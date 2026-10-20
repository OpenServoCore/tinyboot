use log::debug;
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{EraseData, Flags, Frame, InfoData};
use tinyboot_protocol::{COMPAT_UNSET, Cmd, ResetFlags, Status, WriteFlags};

/// Largest payload the host sends; each Write carries the smaller of this
/// and the device's `max_payload`.
const HOST_PAYLOAD: usize = 1024;

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

pub struct Client<T: embedded_io::Read + embedded_io::Write> {
    transport: T,
    frame: Frame<HOST_PAYLOAD>,
}

impl<T: embedded_io::Read + embedded_io::Write> Client<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            frame: Frame::default(),
        }
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

    #[test]
    fn image_compat_reads_a_word_aligned_tail() {
        let mut image = vec![0u8; 10];
        image[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        assert_eq!(image_compat(&image), Some(0x1234_5678));
        assert_eq!(image_compat(&image[..8]), None);
        assert_eq!(image_compat(&image[..2]), None);
    }
}
