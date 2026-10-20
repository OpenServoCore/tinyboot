use std::time::Duration;

use log::debug;
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{EraseData, Flags, Frame, InfoData};
use tinyboot_protocol::{COMPAT_UNSET, Cmd, ResetFlags, Status, WriteFlags};

/// Largest payload the host sends; each Write carries the smaller of this
/// and the device's `max_payload`.
const HOST_PAYLOAD: usize = 1024;

/// How long a read waits for a reply.
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// Longest unique id a device may have, in bytes.
const MAX_UID_LEN: usize = 16;

/// The bus: a byte stream whose read timeout can be set.
pub trait Link: embedded_io::Read + embedded_io::Write {
    fn set_timeout(&mut self, timeout: Duration);
}

/// What one `Select` heard.
#[derive(Debug, PartialEq)]
pub enum Heard {
    Nothing,
    /// One device answered with this id.
    One(Vec<u8>),
    /// A device without addressing answered, refusing `Select`.
    Unaddressed,
    /// Several devices answered at once.
    Garble,
}

/// Devices a [`Client::discover`] found.
#[derive(Debug, PartialEq)]
pub enum Found {
    /// Unique ids, each selectable with its full id.
    Ids(Vec<Vec<u8>>),
    /// One device without addressing.
    Unaddressed,
}

#[derive(Debug, thiserror::Error)]
pub enum FlashError {
    #[error("transport I/O error")]
    Io,
    #[error("device returned error: {0:?}")]
    Device(Status),
    #[error("firmware too large: {size} bytes, device capacity: {capacity} bytes")]
    FirmwareTooLarge { size: u32, capacity: u32 },
    #[error("CRC mismatch: expected {expected:#06X}, got {actual:#06X}")]
    CrcMismatch { expected: u16, actual: u16 },
    #[error("invalid info response")]
    BadInfo,
    #[error(
        "image is not for this device: its compatibility key is {}, the bootloader's is {device:#010X}",
        image.map_or("missing".to_string(), |k| format!("{k:#010X}"))
    )]
    Incompatible { device: u32, image: Option<u32> },
    #[error("no device answered")]
    NoDevice,
    #[error("more than one device answered; pick one with --uid (see `tinyboot discover`)")]
    SeveralDevices,
    #[error("no device with uid {0}")]
    NoSuchUid(String),
    #[error("uid {0} matches more than one device; give more of it")]
    AmbiguousUid(String),
    #[error("the device is not addressable; drop --uid")]
    NotAddressable,
    #[error("two devices answer with one uid")]
    DuplicateUid,
}

/// Lowercase hex of an id, least significant byte first, as stored.
pub fn uid_hex(uid: &[u8]) -> String {
    uid.iter().map(|b| format!("{b:02x}")).collect()
}

/// Parse an id, or a prefix of one, written as [`uid_hex`] prints it.
pub fn parse_uid(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 || hex.len() > 2 * MAX_UID_LEN {
        return Err(format!(
            "bad uid {hex}: whole bytes, 2 hex digits each, at most {MAX_UID_LEN}"
        ));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| format!("bad uid {hex}")))
        .collect()
}

/// Bit `k` of an id is `id[k / 8] >> (k % 8) & 1`.
fn bits_of(bytes: &[u8]) -> Vec<bool> {
    (0..bytes.len() * 8)
        .map(|k| bytes[k / 8] >> (k % 8) & 1 != 0)
        .collect()
}

/// What a probe's bytes hold: replies from one device read clean, from
/// several at once garble. The host's own request, echoed on one-wire
/// setups, is skipped.
fn hear(bytes: &[u8]) -> Heard {
    let mut r = SliceReader { bytes, pos: 0 };
    let mut framed = 0;
    let mut replies = Vec::new();
    while r.pos < bytes.len() {
        let mut f = Frame::<HOST_PAYLOAD>::default();
        match f.read(&mut r) {
            Ok(Status::Ok) => {}
            _ => return Heard::Garble,
        }
        framed += 12 + f.len as usize;
        if f.status != Status::Request {
            replies.push(f);
        }
    }
    // Bytes the sync scan skipped are another device's.
    if framed != bytes.len() {
        return Heard::Garble;
    }
    match replies.as_slice() {
        [] => Heard::Nothing,
        [f] if f.status == Status::Ok => {
            Heard::One(unsafe { f.data.raw[..f.len as usize].to_vec() })
        }
        [f] if f.status == Status::Unsupported => Heard::Unaddressed,
        _ => Heard::Garble,
    }
}

struct SliceReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl embedded_io::ErrorType for SliceReader<'_> {
    type Error = embedded_io::ErrorKind;
}

impl embedded_io::Read for SliceReader<'_> {
    // An error at the end: the sync scan reads on through `Ok(0)`.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.pos == self.bytes.len() {
            return Err(embedded_io::ErrorKind::TimedOut);
        }
        let n = buf.len().min(self.bytes.len() - self.pos);
        buf[..n].copy_from_slice(&self.bytes[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Compatibility key in an image's tail. The tail starts word-aligned, so
/// a tagged image is 2 bytes past a word.
pub fn image_compat(image: &[u8]) -> Option<u32> {
    let n = image.len();
    (n >= 6 && n % 4 == 2).then(|| u32::from_le_bytes(image[n - 6..n - 2].try_into().unwrap()))
}

/// Device geometry from Info response.
#[derive(Debug, Clone, Copy)]
pub struct DeviceInfo {
    pub capacity: u32,
    pub erase_size: u16,
    pub boot_version: u16,
    pub app_version: u16,
    pub mode: u8,
    /// Stored bootloader baud index; `0xFF` = build default.
    pub boot_baud: u8,
    pub compat: u32,
    pub max_payload: u16,
}

pub struct Client<T: Link> {
    transport: T,
    frame: Frame<HOST_PAYLOAD>,
    /// How long a `Select` listens for replies.
    probe_timeout: Duration,
    seed: u8,
    /// Id of the selected device; `None` before a select or when unaddressed.
    uid: Option<Vec<u8>>,
}

impl<T: Link> Client<T> {
    /// `probe_timeout` must cover the longest `Select` reply: a 15
    /// byte-time slot and the reply itself.
    pub fn new(transport: T, probe_timeout: Duration) -> Self {
        Self {
            transport,
            frame: Frame::default(),
            probe_timeout,
            seed: 0,
            uid: None,
        }
    }

    /// Id of the selected device, `None` if it is unaddressed.
    pub fn uid(&self) -> Option<&[u8]> {
        self.uid.as_deref()
    }

    /// Send `Select` for a `prefix` of id bits and listen for replies.
    pub fn select(&mut self, prefix: &[bool]) -> Result<Heard, FlashError> {
        // A fresh slot draw each time, so two devices that answered in
        // unison do not again.
        self.seed = self.seed.wrapping_add(1);
        let n = prefix.len().div_ceil(8);
        let mut data = vec![0u8; 2 + n];
        data[0] = self.seed;
        data[1] = prefix.len() as u8;
        for (k, &b) in prefix.iter().enumerate() {
            data[2 + k / 8] |= (b as u8) << (k % 8);
        }
        self.frame.cmd = Cmd::Select;
        self.frame.set_addr(0);
        self.frame.flags = Flags { raw: 0 };
        self.frame.len = data.len() as u16;
        unsafe { self.frame.data.raw[..data.len()].copy_from_slice(&data) };
        self.frame.status = Status::Request;
        debug!(">> Select seed={} prefix_len={}", self.seed, prefix.len());
        self.frame
            .send(&mut self.transport)
            .map_err(|_| FlashError::Io)?;
        let bytes = self.listen();
        let heard = hear(&bytes);
        debug!("<< {} bytes: {heard:?}", bytes.len());
        Ok(heard)
    }

    /// Every byte until the line stays quiet for `probe_timeout`.
    fn listen(&mut self) -> Vec<u8> {
        self.transport.set_timeout(self.probe_timeout);
        let mut bytes = Vec::new();
        let mut buf = [0u8; 256];
        while let Ok(n @ 1..) = self.transport.read(&mut buf) {
            bytes.extend_from_slice(&buf[..n]);
        }
        self.transport.set_timeout(TIMEOUT);
        bytes
    }

    /// Deselect every device.
    pub fn deselect_all(&mut self) -> Result<(), FlashError> {
        // Longer than any id, so it matches none.
        let none = vec![false; 8 * MAX_UID_LEN + 1];
        self.select(&none)?;
        self.uid = None;
        Ok(())
    }

    /// Select the one device whose id starts with `uid`, or the only device
    /// on the bus when `uid` is `None`. A device without addressing is
    /// used as it is.
    pub fn attach(&mut self, uid: Option<&[u8]>) -> Result<(), FlashError> {
        let prefix = bits_of(uid.unwrap_or_default());
        match self.select(&prefix)? {
            Heard::One(id) => {
                self.uid = Some(id);
                Ok(())
            }
            Heard::Unaddressed if uid.is_none() => {
                self.uid = None;
                Ok(())
            }
            Heard::Unaddressed => Err(FlashError::NotAddressable),
            Heard::Nothing => Err(match uid {
                Some(u) => FlashError::NoSuchUid(uid_hex(u)),
                None => FlashError::NoDevice,
            }),
            Heard::Garble => {
                self.deselect_all()?;
                Err(match uid {
                    Some(u) => FlashError::AmbiguousUid(uid_hex(u)),
                    None => FlashError::SeveralDevices,
                })
            }
        }
    }

    /// Find every device whose id starts with `prefix` by walking the id
    /// tree: a clean reply under a prefix is a candidate, garble descends
    /// one bit, silence ends the branch. Leaves every device deselected.
    pub fn discover(&mut self, prefix: &[u8]) -> Result<Found, FlashError> {
        let mut ids = Vec::new();
        match self.walk(&mut bits_of(prefix), &mut ids, None) {
            Err(FlashError::NotAddressable) => return Ok(Found::Unaddressed),
            r => r?,
        }
        self.deselect_all()?;
        ids.sort();
        Ok(Found::Ids(ids))
    }

    /// `heard`: what this prefix is already known to hold, else probe it.
    fn walk(
        &mut self,
        prefix: &mut Vec<bool>,
        ids: &mut Vec<Vec<u8>>,
        heard: Option<Heard>,
    ) -> Result<(), FlashError> {
        let heard = match heard {
            Some(h) => h,
            None => self.select(prefix)?,
        };
        match heard {
            Heard::Nothing => return Ok(()),
            Heard::Unaddressed => return Err(FlashError::NotAddressable),
            Heard::One(id) if self.confirm(prefix, &id)? => {
                ids.push(id);
                return Ok(());
            }
            _ => {}
        }
        if prefix.len() >= 8 * MAX_UID_LEN {
            return Err(FlashError::DuplicateUid);
        }
        prefix.push(false);
        let zero = self.select(prefix)?;
        // Nothing under 0 leaves every device under 1: skip that probe.
        let one = (zero == Heard::Nothing).then_some(Heard::Garble);
        self.walk(prefix, ids, Some(zero))?;
        prefix.pop();
        prefix.push(true);
        self.walk(prefix, ids, one)?;
        prefix.pop();
        Ok(())
    }

    /// Two devices in unison can read back as one clean reply. A real one
    /// answers alone under the child prefix its id continues with, and
    /// nothing answers under the other.
    fn confirm(&mut self, prefix: &mut Vec<bool>, id: &[u8]) -> Result<bool, FlashError> {
        let bits = bits_of(id);
        if !bits.starts_with(prefix) {
            return Ok(false);
        }
        let Some(&next) = bits.get(prefix.len()) else {
            return Ok(true);
        };
        prefix.push(next);
        let alone = self.select(prefix)? == Heard::One(id.to_vec());
        prefix.pop();
        prefix.push(!next);
        let other = self.select(prefix)? == Heard::Nothing;
        prefix.pop();
        Ok(alone && other)
    }

    /// Send current frame as a request and read the response.
    fn transact(&mut self) -> Result<(), FlashError> {
        self.frame.status = Status::Request;
        debug!(
            ">> {:?} addr={:#X} len={}",
            self.frame.cmd,
            self.frame.addr(),
            self.frame.len
        );
        self.frame
            .send(&mut self.transport)
            .map_err(|_| FlashError::Io)?;

        // On single-wire setups like the OSC-Dev-V006 board — where MCU TX/RX
        // and the host's TX/RX all land on the same DXL-TTL data line — the
        // host hears its own request frame before the device replies. Skip
        // frames whose status is `Request`: those can only be our own echo,
        // since devices never reply with that status.
        let parse_status = loop {
            let s = self
                .frame
                .read(&mut self.transport)
                .map_err(|_| FlashError::Io)?;
            if s == Status::Ok && self.frame.status == Status::Request {
                continue;
            }
            break s;
        };

        debug!(
            "<< {:?} status={:?} len={}",
            self.frame.cmd, self.frame.status, self.frame.len
        );
        if parse_status != Status::Ok {
            return Err(FlashError::Device(parse_status));
        }
        match self.frame.status {
            Status::Ok => Ok(()),
            status => Err(FlashError::Device(status)),
        }
    }

    /// Query device geometry.
    pub fn info(&mut self) -> Result<DeviceInfo, FlashError> {
        self.frame.cmd = Cmd::Info;
        self.frame.set_addr(0);
        self.frame.flags = Flags { raw: 0 };
        self.frame.len = 0;
        self.transact()?;

        if self.frame.len < InfoData::SIZE {
            return Err(FlashError::BadInfo);
        }

        let info = unsafe { self.frame.data.info };
        let capacity = { info.capacity };
        let erase_size = { info.erase_size };
        let boot_version = { info.boot_version };
        let app_version = { info.app_version };
        let mode = { info.mode };
        let boot_baud = { info.boot_baud };
        let compat = { info.compat };
        let max_payload = { info.max_payload };

        if erase_size == 0 || capacity == 0 || max_payload < 4 {
            return Err(FlashError::BadInfo);
        }

        Ok(DeviceInfo {
            capacity,
            erase_size,
            boot_version,
            app_version,
            mode,
            boot_baud,
            compat,
            max_payload,
        })
    }

    /// Erase entire app region.
    pub fn erase(
        &mut self,
        on_progress: &mut dyn FnMut(u32, u32),
    ) -> Result<DeviceInfo, FlashError> {
        let info = self.info()?;
        let erase_size = info.erase_size as u32;
        let capacity = info.capacity;
        let mut addr = 0u32;
        while addr < capacity {
            self.frame.cmd = Cmd::Erase;
            self.frame.set_addr(addr);
            self.frame.flags = Flags { raw: 0 };
            self.frame.len = 2;
            self.frame.data.erase = EraseData {
                byte_count: erase_size as u16,
            };
            self.transact()?;
            addr += erase_size;
            on_progress(addr, capacity);
        }
        Ok(info)
    }

    /// Flash firmware to device.
    ///
    /// Calls `on_progress(phase, current, total)` for progress reporting.
    pub fn flash(
        &mut self,
        firmware: &[u8],
        on_progress: &mut dyn FnMut(&str, u32, u32),
    ) -> Result<DeviceInfo, FlashError> {
        // 1. Info
        let info = self.info()?;

        let fw_size = firmware.len() as u32;
        if fw_size > info.capacity {
            return Err(FlashError::FirmwareTooLarge {
                size: fw_size,
                capacity: info.capacity,
            });
        }

        // A wrong image is refused before the erase destroys a working app.
        if info.compat != COMPAT_UNSET && image_compat(firmware) != Some(info.compat) {
            return Err(FlashError::Incompatible {
                device: info.compat,
                image: image_compat(firmware),
            });
        }

        let erase_size = info.erase_size as u32;

        // 2. Erase — full region, page by page
        let mut erase_addr = 0u32;
        while erase_addr < info.capacity {
            self.frame.cmd = Cmd::Erase;
            self.frame.set_addr(erase_addr);
            self.frame.flags = Flags { raw: 0 };
            self.frame.len = 2;
            self.frame.data.erase = EraseData {
                byte_count: erase_size as u16,
            };
            self.transact()?;
            erase_addr += erase_size;
            on_progress("Erasing", erase_addr, info.capacity);
        }

        // 3. Write — chunk by the smaller payload capacity, pad to 4-byte
        //    alignment with 0xFF. Set FLUSH flag on the last chunk.
        let chunk = HOST_PAYLOAD.min(info.max_payload as usize) & !3;
        let padded_len = firmware.len().next_multiple_of(4);
        let total_chunks = padded_len.div_ceil(chunk) as u32;
        let mut offset = 0usize;
        let mut chunk_idx = 0u32;
        while offset < padded_len {
            let end = (offset + chunk).min(padded_len);
            let len = end - offset;
            let is_last = end >= padded_len;
            // Copy firmware bytes, pad remainder with 0xFF
            let fw_end = end.min(firmware.len());
            let fw_bytes = fw_end.saturating_sub(offset);
            unsafe {
                self.frame.data.raw[..fw_bytes]
                    .copy_from_slice(&firmware[offset..offset + fw_bytes]);
                self.frame.data.raw[fw_bytes..len].fill(0xFF);
            }
            self.frame.cmd = Cmd::Write;
            self.frame.set_addr(offset as u32);
            self.frame.flags = Flags {
                write: if is_last {
                    WriteFlags::FLUSH
                } else {
                    WriteFlags::empty()
                },
            };
            self.frame.len = len as u16;
            match self.transact() {
                Ok(()) => {}
                Err(FlashError::Device(Status::CrcMismatch)) => {
                    // Response frame corrupted — retry from current offset.
                    debug!("CRC mismatch at {offset:#X}, retrying");
                    continue;
                }
                Err(e) => return Err(e),
            }
            offset = end;
            chunk_idx += 1;
            on_progress("Writing", chunk_idx, total_chunks);
        }

        // 4. Verify — CRC covers original firmware bytes only (not padding)
        let expected_crc = crc16(CRC_INIT, firmware);

        self.frame.cmd = Cmd::Verify;
        self.frame.set_addr(fw_size);
        self.frame.flags = Flags { raw: 0 };
        self.frame.len = 0;
        self.transact()?;

        let actual_crc = unsafe { self.frame.data.verify }.crc;
        if actual_crc != expected_crc {
            return Err(FlashError::CrcMismatch {
                expected: expected_crc,
                actual: actual_crc,
            });
        }

        Ok(info)
    }

    /// Reset the device into the bootloader or the app. A bootloader
    /// replies before it resets, so `in_boot` waits for the reply and
    /// reports a refusal; an app resets without replying.
    pub fn reset(&mut self, bootloader: bool, in_boot: bool) -> Result<(), FlashError> {
        self.send_reset(bootloader, None, in_boot)
    }

    /// Store `baud` (a `BaudRate` index; `0xFF` = the build default) for
    /// the bootloader, then reset as [`reset`](Self::reset).
    pub fn set_baud(
        &mut self,
        baud: u8,
        bootloader: bool,
        in_boot: bool,
    ) -> Result<(), FlashError> {
        self.send_reset(bootloader, Some(baud), in_boot)
    }

    fn send_reset(
        &mut self,
        bootloader: bool,
        baud: Option<u8>,
        in_boot: bool,
    ) -> Result<(), FlashError> {
        let mut flags = ResetFlags::empty();
        flags.set(ResetFlags::BOOTLOADER, bootloader);
        flags.set(ResetFlags::BAUD, baud.is_some());
        self.frame.cmd = Cmd::Reset;
        self.frame.set_addr(baud.unwrap_or(0) as u32);
        self.frame.flags = Flags { reset: flags };
        self.frame.len = 0;
        if in_boot {
            return self.transact();
        }
        self.frame.status = Status::Request;
        let _ = self.frame.send(&mut self.transport);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Devices on one wire. Replies to a `Select` from two or more devices
    /// overlap and arrive as their bitwise AND, as on a wired-AND line.
    struct Bus {
        /// `(id, selected)`; an empty id is a device without addressing.
        devices: Vec<(Vec<u8>, bool)>,
        /// Request bytes so far: `send` writes the body, then the CRC.
        tx: Vec<u8>,
        rx: Vec<u8>,
        selects: usize,
    }

    impl Bus {
        fn new(ids: &[&[u8]]) -> Self {
            Self {
                devices: ids.iter().map(|id| (id.to_vec(), false)).collect(),
                tx: Vec::new(),
                rx: Vec::new(),
                selects: 0,
            }
        }

        fn reply(cmd: Cmd, status: Status, data: &[u8]) -> Vec<u8> {
            let mut f = Frame::<64>::default();
            f.cmd = cmd;
            f.status = status;
            f.len = data.len() as u16;
            unsafe { f.data.raw[..data.len()].copy_from_slice(data) };
            let mut out = Wire(Vec::new());
            f.send(&mut out).unwrap();
            out.0
        }
    }

    struct Wire(Vec<u8>);

    impl embedded_io::ErrorType for Wire {
        type Error = core::convert::Infallible;
    }

    impl embedded_io::Write for Wire {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    impl embedded_io::ErrorType for Bus {
        type Error = embedded_io::ErrorKind;
    }

    impl embedded_io::Write for Bus {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.tx.extend_from_slice(buf);
            let t = &self.tx;
            if t.len() < 10 || t.len() < 12 + u16::from_le_bytes([t[8], t[9]]) as usize {
                return Ok(buf.len());
            }
            let req = std::mem::take(&mut self.tx);
            let mut f = Frame::<64>::default();
            assert_eq!(
                f.read(&mut SliceReader {
                    bytes: &req,
                    pos: 0
                }),
                Ok(Status::Ok)
            );
            self.selects += (f.cmd == Cmd::Select) as usize;
            let mut replies = Vec::new();
            for (id, selected) in &mut self.devices {
                if id.is_empty() {
                    replies.push(Self::reply(f.cmd, Status::Unsupported, &[]));
                    continue;
                }
                if f.cmd != Cmd::Select {
                    if *selected {
                        replies.push(Self::reply(f.cmd, Status::Ok, &[0; 18]));
                    }
                    continue;
                }
                let data = unsafe { &f.data.raw };
                let n = data[1] as usize;
                let bits = bits_of(id);
                *selected = n <= bits.len()
                    && (0..n).all(|k| bits[k] == (data[2 + k / 8] >> (k % 8) & 1 != 0));
                if *selected {
                    replies.push(Self::reply(Cmd::Select, Status::Ok, id));
                }
            }
            let len = replies.iter().map(Vec::len).max().unwrap_or(0);
            let mut wire = vec![0xFF; len];
            for r in &replies {
                for (w, b) in wire.iter_mut().zip(r) {
                    *w &= b;
                }
            }
            self.rx = wire;
            Ok(buf.len())
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    impl embedded_io::Read for Bus {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            if self.rx.is_empty() {
                return Err(embedded_io::ErrorKind::TimedOut);
            }
            let n = buf.len().min(self.rx.len());
            buf[..n].copy_from_slice(&self.rx[..n]);
            self.rx.drain(..n);
            Ok(n)
        }
    }

    impl Link for Bus {
        fn set_timeout(&mut self, _: Duration) {}
    }

    const IDS: [[u8; 12]; 5] = [
        [
            0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, 0x01, 0x23, 0x45, 0x07,
        ],
        [
            0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, 0x01, 0x23, 0x45, 0x87,
        ],
        [0; 12],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0xFF; 12],
    ];

    fn client(ids: &[&[u8]]) -> Client<Bus> {
        Client::new(Bus::new(ids), Duration::ZERO)
    }

    #[test]
    fn discover_finds_every_id_and_deselects_them() {
        let ids: Vec<&[u8]> = IDS.iter().map(|i| &i[..]).collect();
        let mut c = client(&ids);
        let Found::Ids(found) = c.discover(&[]).unwrap() else {
            panic!("unaddressed");
        };
        let mut want: Vec<Vec<u8>> = IDS.iter().map(|i| i.to_vec()).collect();
        want.sort();
        assert_eq!(found, want);
        assert!(c.transport.devices.iter().all(|(_, s)| !s));
    }

    #[test]
    fn discover_descends_a_shared_prefix_in_two_probes_a_bit_at_most() {
        // These two differ only in their last bit: 95 shared bits.
        let mut c = client(&[&IDS[0], &IDS[1]]);
        assert_eq!(
            c.discover(&[]).unwrap(),
            Found::Ids(vec![IDS[0].to_vec(), IDS[1].to_vec()])
        );
        let probes = c.transport.selects;
        // A shared 1 bit costs one probe, a shared 0 bit two.
        assert!(probes <= 2 * 96, "{probes} probes");
    }

    #[test]
    fn discover_under_a_prefix_finds_only_that_subtree() {
        let ids: Vec<&[u8]> = IDS.iter().map(|i| &i[..]).collect();
        let mut c = client(&ids);
        let Found::Ids(found) = c.discover(&[0x10, 0x32]).unwrap() else {
            panic!("unaddressed");
        };
        assert_eq!(found, vec![IDS[0].to_vec(), IDS[1].to_vec()]);
    }

    #[test]
    fn discover_reports_a_device_without_addressing() {
        let mut c = client(&[&[]]);
        assert_eq!(c.discover(&[]).unwrap(), Found::Unaddressed);
    }

    #[test]
    fn attach_selects_the_only_device() {
        let mut c = client(&[&IDS[2]]);
        c.attach(None).unwrap();
        assert_eq!(c.uid(), Some(&IDS[2][..]));
        assert!(c.transport.devices[0].1);
    }

    #[test]
    fn attach_without_uid_refuses_several_devices() {
        let mut c = client(&[&IDS[0], &IDS[2]]);
        assert!(matches!(c.attach(None), Err(FlashError::SeveralDevices)));
        assert!(c.transport.devices.iter().all(|(_, s)| !s));
    }

    #[test]
    fn attach_by_prefix_selects_one_and_deselects_the_rest() {
        let mut c = client(&[&IDS[0], &IDS[2], &IDS[4]]);
        c.attach(Some(&[0xFF])).unwrap();
        assert_eq!(c.uid(), Some(&IDS[4][..]));
        let selected: Vec<bool> = c.transport.devices.iter().map(|d| d.1).collect();
        assert_eq!(selected, [false, false, true]);
        assert!(matches!(
            c.attach(Some(&[0x10, 0x32])),
            Ok(()) if c.uid() == Some(&IDS[0][..])
        ));
    }

    #[test]
    fn attach_names_a_missing_or_ambiguous_uid() {
        let mut c = client(&[&IDS[0], &IDS[1]]);
        assert!(matches!(c.attach(Some(&[0x11])), Err(FlashError::NoSuchUid(u)) if u == "11"));
        assert!(matches!(
            c.attach(Some(&[0x10])),
            Err(FlashError::AmbiguousUid(_))
        ));
        assert!(matches!(
            client(&[]).attach(None),
            Err(FlashError::NoDevice)
        ));
        assert!(matches!(
            client(&[&[]]).attach(Some(&[1])),
            Err(FlashError::NotAddressable)
        ));
    }

    #[test]
    fn attach_uses_a_device_without_addressing_as_it_is() {
        let mut c = client(&[&[]]);
        c.attach(None).unwrap();
        assert_eq!(c.uid(), None);
    }

    #[test]
    fn hear_skips_the_echo_and_flags_trailing_bytes() {
        let echo = Bus::reply(Cmd::Select, Status::Request, &[1, 0]);
        let one = Bus::reply(Cmd::Select, Status::Ok, &IDS[0]);
        assert_eq!(
            hear(&[echo.clone(), one.clone()].concat()),
            Heard::One(IDS[0].to_vec())
        );
        assert_eq!(hear(&echo), Heard::Nothing);
        assert_eq!(hear(&[one.clone(), vec![0x00]].concat()), Heard::Garble);
        assert_eq!(hear(&[vec![0x13], one.clone()].concat()), Heard::Garble);
        assert_eq!(hear(&[one.clone(), one].concat()), Heard::Garble);
    }

    #[test]
    fn uid_hex_round_trips() {
        assert_eq!(uid_hex(&IDS[0]), "1032547698badcfe01234507");
        assert_eq!(parse_uid("1032547698badcfe01234507").unwrap(), IDS[0]);
        assert_eq!(parse_uid("10AB").unwrap(), [0x10, 0xAB]);
        assert!(parse_uid("103").is_err());
        assert!(parse_uid("zz").is_err());
    }

    #[test]
    fn image_compat_reads_a_word_aligned_tail() {
        let mut image = vec![0u8; 10];
        image[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        assert_eq!(image_compat(&image), Some(0x1234_5678));
        assert_eq!(image_compat(&image[..8]), None);
        assert_eq!(image_compat(&image[..2]), None);
    }
}
