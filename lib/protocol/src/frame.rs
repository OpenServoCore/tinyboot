use core::mem::MaybeUninit;

use crate::crc::{CRC_INIT, crc16};
use crate::sync::Sync;
use crate::{Cmd, ReadError, ResetFlags, Status, WriteFlags};

/// Default data capacity of a [`Frame`], the largest payload a request may
/// carry to a device built with it. Info reports a device's capacity.
/// With the 12 bytes of framing, a full frame fits one 64-byte USB
/// full-speed packet.
pub const DEFAULT_PAYLOAD: usize = 52;

/// Typed Info response data.
///
/// Packed to keep alignment ≤ 2 so the `Data` union doesn't force padding
/// inside `Frame` (data starts at offset 10, not 4-byte aligned).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct InfoData {
    /// App region capacity in bytes.
    pub capacity: u32,
    /// Erase page size in bytes.
    pub erase_size: u16,
    /// Boot version (packed 5.5.6, `0xFFFF` = none).
    pub boot_version: u16,
    /// App version (packed 5.5.6, `0xFFFF` = none).
    pub app_version: u16,
    /// 0 = bootloader, 1 = app.
    pub mode: u8,
    /// Stored bootloader [`BaudRate`](crate::BaudRate) index; `0xFF` = the
    /// bootloader's build default.
    pub boot_baud: u8,
    /// The bootloader's compatibility key
    /// ([`COMPAT_UNSET`](crate::COMPAT_UNSET) = accepts any image).
    pub compat: u32,
    /// Largest payload the responder accepts in a request.
    pub max_payload: u16,
}

impl InfoData {
    /// Encoded size in bytes.
    pub const SIZE: u16 = core::mem::size_of::<Self>() as u16;

    /// Decode from the start of `bytes` (little-endian fields in declaration
    /// order); `None` when `bytes` is shorter than [`SIZE`](Self::SIZE).
    /// Bytes past `SIZE` are ignored, so a longer reply from a newer device
    /// still decodes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..Self::SIZE as usize)?;
        Some(Self {
            capacity: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            erase_size: u16::from_le_bytes([b[4], b[5]]),
            boot_version: u16::from_le_bytes([b[6], b[7]]),
            app_version: u16::from_le_bytes([b[8], b[9]]),
            mode: b[10],
            boot_baud: b[11],
            compat: u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
            max_payload: u16::from_le_bytes([b[16], b[17]]),
        })
    }

    /// Encode as little-endian fields in declaration order.
    pub fn to_bytes(&self) -> [u8; Self::SIZE as usize] {
        let mut b = [0; Self::SIZE as usize];
        b[0..4].copy_from_slice(&{ self.capacity }.to_le_bytes());
        b[4..6].copy_from_slice(&{ self.erase_size }.to_le_bytes());
        b[6..8].copy_from_slice(&{ self.boot_version }.to_le_bytes());
        b[8..10].copy_from_slice(&{ self.app_version }.to_le_bytes());
        b[10] = self.mode;
        b[11] = self.boot_baud;
        b[12..16].copy_from_slice(&{ self.compat }.to_le_bytes());
        b[16..18].copy_from_slice(&{ self.max_payload }.to_le_bytes());
        b
    }
}

/// Typed Erase request data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EraseData {
    /// Number of bytes to erase (must be aligned to erase size).
    pub byte_count: u16,
}

impl EraseData {
    /// Encoded size in bytes.
    pub const SIZE: u16 = core::mem::size_of::<Self>() as u16;

    /// Decode from the start of `bytes` (little-endian fields in declaration
    /// order); `None` when `bytes` is shorter than [`SIZE`](Self::SIZE).
    /// Bytes past `SIZE` are ignored, so a longer reply from a newer device
    /// still decodes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..Self::SIZE as usize)?;
        Some(Self {
            byte_count: u16::from_le_bytes([b[0], b[1]]),
        })
    }

    /// Encode as little-endian fields in declaration order.
    pub fn to_bytes(&self) -> [u8; Self::SIZE as usize] {
        self.byte_count.to_le_bytes()
    }
}

/// Typed Verify response data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VerifyData {
    /// CRC16 of the app region.
    pub crc: u16,
}

impl VerifyData {
    /// Encoded size in bytes.
    pub const SIZE: u16 = core::mem::size_of::<Self>() as u16;

    /// Decode from the start of `bytes` (little-endian fields in declaration
    /// order); `None` when `bytes` is shorter than [`SIZE`](Self::SIZE).
    /// Bytes past `SIZE` are ignored, so a longer reply from a newer device
    /// still decodes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..Self::SIZE as usize)?;
        Some(Self {
            crc: u16::from_le_bytes([b[0], b[1]]),
        })
    }

    /// Encode as little-endian fields in declaration order.
    pub fn to_bytes(&self) -> [u8; Self::SIZE as usize] {
        self.crc.to_le_bytes()
    }
}

/// Per-command flags occupying addr byte 3.
///
/// Each command can define its own bitflags type as a union variant.
/// All variants are `u8` so the union is always 1 byte.
#[repr(C)]
#[derive(Clone, Copy)]
pub union Flags {
    /// Raw byte access.
    pub raw: u8,
    /// [`Cmd::Write`] flags.
    pub write: WriteFlags,
    /// [`Cmd::Reset`] flags.
    pub reset: ResetFlags,
}

/// Union-typed data payload.
///
/// Provides zero-cost typed access to frame data. Reading fields is unsafe
/// because the caller must know which variant is active.
#[repr(C)]
pub union Data<const N: usize = DEFAULT_PAYLOAD> {
    /// Raw byte access.
    pub raw: [u8; N],
    /// Info response fields.
    pub info: InfoData,
    /// Erase request fields.
    pub erase: EraseData,
    /// Verify response fields.
    pub verify: VerifyData,
}

/// Wire frame: SYNC(2) + CMD(1) + STATUS(1) + ADDR(4) + LEN(2) + DATA(len) + CRC(2)
///
/// Used for both requests and responses. For requests, `status` is
/// [`Status::Request`]. For responses, `cmd` and `addr` echo the request.
///
/// `N` is the data capacity: the largest payload this side reads.
///
/// Single instance, reused each iteration of the protocol loop.
#[repr(C)]
pub struct Frame<const N: usize = DEFAULT_PAYLOAD> {
    sync: Sync,
    /// Command code.
    pub cmd: Cmd,
    /// Response status (always [`Status::Request`] for requests).
    pub status: Status,
    /// Address bits [15:0].
    pub addr_lo: u16,
    /// Address bits [23:16].
    pub addr_hi: u8,
    /// Per-command flags (addr byte 3).
    pub flags: Flags,
    /// Data payload length in bytes (0..=N).
    pub len: u16,
    /// Payload data (union-typed).
    pub data: Data<N>,
    /// CRC16 over the frame body (little-endian).
    pub crc: [u8; 2],
}

impl<const N: usize> Default for Frame<N> {
    /// Create a default frame. Data buffer is uninitialized — `read()` or
    /// caller writes populate it before use.
    ///
    /// Header fields are written in place through a raw pointer, so `data`
    /// stays uninit (valid for a union) and is never copied: a whole-value
    /// `assume_init` lets LLVM move the payload with a 64-byte `memcpy`.
    fn default() -> Self {
        let mut frame = MaybeUninit::<Self>::uninit();
        let p = frame.as_mut_ptr();
        unsafe {
            (&raw mut (*p).sync).write(Sync::default());
            (&raw mut (*p).cmd).write(Cmd::Info);
            (&raw mut (*p).status).write(Status::Request);
            (&raw mut (*p).addr_lo).write(0);
            (&raw mut (*p).addr_hi).write(0);
            (&raw mut (*p).flags).write(Flags { raw: 0 });
            (&raw mut (*p).len).write(0);
            (&raw mut (*p).crc).write([0; 2]);
            frame.assume_init()
        }
    }
}

impl<const N: usize> Frame<N> {
    /// Reconstruct the 24-bit address from lo + hi.
    #[inline(always)]
    pub fn addr(&self) -> u32 {
        self.addr_lo as u32 | (self.addr_hi as u32) << 16
    }

    /// Set the 24-bit address (lo + hi). Does not touch flags.
    #[inline(always)]
    pub fn set_addr(&mut self, addr: u32) {
        self.addr_lo = addr as u16;
        self.addr_hi = (addr >> 16) as u8;
    }
}

impl<const N: usize> Frame<N> {
    fn as_bytes(&self, offset: usize, len: usize) -> &[u8] {
        debug_assert!(offset + len <= core::mem::size_of::<Self>());
        unsafe {
            let ptr = (self as *const Self as *const u8).add(offset);
            core::slice::from_raw_parts(ptr, len)
        }
    }

    fn as_bytes_mut(&mut self, offset: usize, len: usize) -> &mut [u8] {
        debug_assert!(offset + len <= core::mem::size_of::<Self>());
        unsafe {
            let ptr = (self as *mut Self as *mut u8).add(offset);
            core::slice::from_raw_parts_mut(ptr, len)
        }
    }

    /// Send the frame. CRC is placed inline after data for a single write.
    pub fn send<W: embedded_io::Write>(&mut self, w: &mut W) -> Result<(), W::Error> {
        self.sync = Sync::valid();
        let body_len = 10 + self.len as usize;
        let crc = crc16(CRC_INIT, self.as_bytes(0, body_len)).to_le_bytes();
        // Place CRC immediately after payload for a contiguous write.
        // SAFETY: Frame is #[repr(C)] so offset 10+len is within the struct
        // (inside `data` when len < N, or at or before `crc` when len == N).
        unsafe {
            let base = self as *mut Self as *mut u8;
            *base.add(10 + self.len as usize) = crc[0];
            *base.add(10 + self.len as usize + 1) = crc[1];
        }
        w.write_all(self.as_bytes(0, body_len + 2))
    }

    /// Read one frame from the transport.
    ///
    /// Syncs on preamble, reads header + payload, validates CRC.
    /// Returns `Ok(Status::Ok)` on success, `Ok(Status::*)` for protocol
    /// errors (CRC, invalid frame, overflow), `Err` only for transport IO.
    pub fn read<R: embedded_io::Read>(&mut self, r: &mut R) -> Result<Status, ReadError> {
        self.sync.read(r)?;

        // Read header fields: cmd(1) + status(1) + addr(4) + len(2) = 8 bytes at offset 2
        r.read_exact(self.as_bytes_mut(2, 8))
            .map_err(|_| ReadError)?;

        if !Cmd::is_valid(self.as_bytes(2, 1)[0]) || !Status::is_valid(self.as_bytes(3, 1)[0]) {
            // Remaining payload + CRC bytes are left in the transport. The sync
            // scanner will skip them as garbage on the next read(). Draining here
            // is not feasible: the len field is untrusted and the payload could be
            // up to 64 KB, which exceeds our buffer and time budget on small MCUs.
            return Ok(Status::Unsupported);
        }

        let data_len = self.len as usize;

        if data_len > N {
            // Same as above — we cannot drain a payload larger than our buffer.
            // The sync scanner recovers on the next frame.
            return Ok(Status::PayloadOverflow);
        }

        // Read data directly into buffer
        if data_len > 0 {
            r.read_exact(unsafe { &mut self.data.raw[..data_len] })
                .map_err(|_| ReadError)?;
        }

        // Read CRC directly — [u8; 2] has no alignment constraint
        r.read_exact(&mut self.crc).map_err(|_| ReadError)?;

        // Validate CRC over body
        if self.crc != crc16(CRC_INIT, self.as_bytes(0, 10 + data_len)).to_le_bytes() {
            return Ok(Status::CrcMismatch);
        }

        Ok(Status::Ok)
    }

    /// Async version of [`send`](Self::send).
    pub async fn send_async<W: embedded_io_async::Write>(
        &mut self,
        w: &mut W,
    ) -> Result<(), W::Error> {
        self.sync = Sync::valid();
        let body_len = 10 + self.len as usize;
        self.crc = crc16(CRC_INIT, self.as_bytes(0, body_len)).to_le_bytes();
        w.write_all(self.as_bytes(0, body_len)).await?;
        w.write_all(&self.crc).await
    }

    /// Async version of [`read`](Self::read).
    pub async fn read_async<R: embedded_io_async::Read>(
        &mut self,
        r: &mut R,
    ) -> Result<Status, ReadError> {
        self.sync.read_async(r).await?;

        r.read_exact(self.as_bytes_mut(2, 8))
            .await
            .map_err(|_| ReadError)?;

        if !Cmd::is_valid(self.as_bytes(2, 1)[0]) || !Status::is_valid(self.as_bytes(3, 1)[0]) {
            // See sync read() for rationale on not draining here.
            return Ok(Status::Unsupported);
        }

        let data_len = self.len as usize;

        if data_len > N {
            // See sync read() for rationale on not draining here.
            return Ok(Status::PayloadOverflow);
        }

        if data_len > 0 {
            r.read_exact(unsafe { &mut self.data.raw[..data_len] })
                .await
                .map_err(|_| ReadError)?;
        }

        r.read_exact(&mut self.crc).await.map_err(|_| ReadError)?;

        if self.crc != crc16(CRC_INIT, self.as_bytes(0, 10 + data_len)).to_le_bytes() {
            return Ok(Status::CrcMismatch);
        }

        Ok(Status::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockReader<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl<'a> MockReader<'a> {
        fn new(data: &'a [u8]) -> Self {
            Self { data, pos: 0 }
        }
    }

    impl embedded_io::ErrorType for MockReader<'_> {
        type Error = core::convert::Infallible;
    }

    impl embedded_io::Read for MockReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let n = buf.len().min(self.data.len() - self.pos);
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }

    struct Sink {
        buf: [u8; 512],
        pos: usize,
    }

    impl Sink {
        fn new() -> Self {
            Self {
                buf: [0; 512],
                pos: 0,
            }
        }
        fn written(&self) -> &[u8] {
            &self.buf[..self.pos]
        }
    }

    impl embedded_io::ErrorType for Sink {
        type Error = core::convert::Infallible;
    }

    impl embedded_io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            let n = buf.len().min(self.buf.len() - self.pos);
            self.buf[self.pos..self.pos + n].copy_from_slice(&buf[..n]);
            self.pos += n;
            Ok(n)
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn frame(cmd: Cmd, status: Status, addr: u32, data: &[u8]) -> Frame {
        let mut f = Frame {
            cmd,
            status,
            ..Default::default()
        };
        f.set_addr(addr);
        f.len = data.len() as u16;
        unsafe { f.data.raw[..data.len()].copy_from_slice(data) };
        f
    }

    #[test]
    fn request_round_trip() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0800, &[0xDE, 0xAD]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Write);
        assert_eq!(frame2.len, 2);
        assert_eq!(frame2.addr(), 0x0800);
        assert_eq!(frame2.status, Status::Request);
        assert_eq!(unsafe { &frame2.data.raw[..2] }, &[0xDE, 0xAD]);
    }

    #[test]
    fn response_round_trip() {
        let mut frame = frame(Cmd::Verify, Status::Ok, 0, &[0x12, 0x34]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Verify);
        assert_eq!(frame2.status, Status::Ok);
        assert_eq!(unsafe { &frame2.data.raw[..2] }, &[0x12, 0x34]);
    }

    #[test]
    fn request_no_data() {
        let mut frame = frame(Cmd::Erase, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        // Frame: SYNC(2) + CMD(1) + STATUS(1) + ADDR(4) + LEN(2) + CRC(2) = 12
        assert_eq!(sink.written().len(), 12);

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Erase);
        assert_eq!(frame2.len, 0);
    }

    #[test]
    fn large_addr_round_trip() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0001_0800, &[0xAB]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.addr(), 0x0001_0800);
    }

    #[test]
    fn cmd_addr_carry_over() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0400, &[0xAB, 0xCD]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        // "Device" reads the request
        let mut dev = Frame::<DEFAULT_PAYLOAD>::default();
        dev.read(&mut MockReader::new(sink.written())).unwrap();

        // Device sends response — cmd and addr carry over
        dev.status = Status::Ok;
        dev.len = 0;
        let mut resp_sink = Sink::new();
        dev.send(&mut resp_sink).unwrap();

        // Host reads response
        let mut host = Frame::<DEFAULT_PAYLOAD>::default();
        host.read(&mut MockReader::new(resp_sink.written()))
            .unwrap();
        assert_eq!(host.cmd, Cmd::Write);
        assert_eq!(host.addr(), 0x0400);
        assert_eq!(host.status, Status::Ok);
    }

    #[test]
    fn info_is_18_bytes() {
        assert_eq!(InfoData::SIZE, 18);
    }

    fn union_bytes<const L: usize>(d: &Data) -> [u8; L] {
        let mut b = [0; L];
        b.copy_from_slice(unsafe { &d.raw[..L] });
        b
    }

    fn info() -> InfoData {
        InfoData {
            capacity: 0x0403_0201,
            erase_size: 0x0605,
            boot_version: 0x0807,
            app_version: 0x0A09,
            mode: 0x0B,
            boot_baud: 0x0C,
            compat: 0x100F_0E0D,
            max_payload: 0x1211,
        }
    }

    fn assert_info_eq(a: &InfoData, b: &InfoData) {
        assert_eq!({ a.capacity }, { b.capacity });
        assert_eq!({ a.erase_size }, { b.erase_size });
        assert_eq!({ a.boot_version }, { b.boot_version });
        assert_eq!({ a.app_version }, { b.app_version });
        assert_eq!(a.mode, b.mode);
        assert_eq!(a.boot_baud, b.boot_baud);
        assert_eq!({ a.compat }, { b.compat });
        assert_eq!({ a.max_payload }, { b.max_payload });
    }

    #[test]
    fn info_bytes_match_union() {
        let d = Data { info: info() };
        let raw = union_bytes::<{ InfoData::SIZE as usize }>(&d);
        assert_info_eq(&InfoData::from_bytes(&raw).unwrap(), &info());
        assert_eq!(info().to_bytes(), raw);
    }

    #[test]
    fn info_from_bytes_length() {
        let raw = info().to_bytes();
        assert!(InfoData::from_bytes(&raw[..raw.len() - 1]).is_none());
        let mut long = [0xEE; InfoData::SIZE as usize + 3];
        long[..raw.len()].copy_from_slice(&raw);
        assert_info_eq(&InfoData::from_bytes(&long).unwrap(), &info());
    }

    #[test]
    fn erase_bytes_match_union() {
        let d = Data {
            erase: EraseData { byte_count: 0x0201 },
        };
        let raw = union_bytes::<{ EraseData::SIZE as usize }>(&d);
        assert_eq!(EraseData::from_bytes(&raw).unwrap().byte_count, 0x0201);
        assert_eq!(EraseData { byte_count: 0x0201 }.to_bytes(), raw);
    }

    #[test]
    fn erase_from_bytes_length() {
        assert_eq!(EraseData::SIZE, 2);
        assert!(EraseData::from_bytes(&[0x01]).is_none());
        let long = [0x01, 0x02, 0xEE, 0xEE, 0xEE];
        assert_eq!(EraseData::from_bytes(&long).unwrap().byte_count, 0x0201);
    }

    #[test]
    fn verify_bytes_match_union() {
        let d = Data {
            verify: VerifyData { crc: 0x0201 },
        };
        let raw = union_bytes::<{ VerifyData::SIZE as usize }>(&d);
        assert_eq!(VerifyData::from_bytes(&raw).unwrap().crc, 0x0201);
        assert_eq!(VerifyData { crc: 0x0201 }.to_bytes(), raw);
    }

    #[test]
    fn verify_from_bytes_length() {
        assert_eq!(VerifyData::SIZE, 2);
        assert!(VerifyData::from_bytes(&[0x01]).is_none());
        let long = [0x01, 0x02, 0xEE, 0xEE, 0xEE];
        assert_eq!(VerifyData::from_bytes(&long).unwrap().crc, 0x0201);
    }

    #[test]
    fn large_payload_round_trip() {
        let data = [0x3C; 240];
        let mut f = Frame::<240> {
            cmd: Cmd::Write,
            ..Default::default()
        };
        f.len = 240;
        unsafe { f.data.raw.copy_from_slice(&data) };
        let mut sink = Sink::new();
        f.send(&mut sink).unwrap();
        assert_eq!(sink.written().len(), 12 + 240);

        let mut big = Frame::<240>::default();
        assert_eq!(
            big.read(&mut MockReader::new(sink.written())),
            Ok(Status::Ok)
        );
        assert_eq!(unsafe { big.data.raw }, data);

        // A device with the default capacity refuses it.
        let mut small = Frame::<DEFAULT_PAYLOAD>::default();
        assert_eq!(
            small.read(&mut MockReader::new(sink.written())),
            Ok(Status::PayloadOverflow)
        );
    }

    #[test]
    fn read_bad_cmd() {
        let mut frame = frame(Cmd::Info, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();
        sink.buf[2] ^= 0xFF; // corrupt CMD byte

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        assert_eq!(
            frame2.read(&mut MockReader::new(sink.written())),
            Ok(Status::Unsupported)
        );
    }

    #[test]
    fn read_after_garbage() {
        let mut frame = frame(Cmd::Verify, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();
        let frame_len = sink.pos;

        let mut input = [0u8; 4 + 512];
        input[..4].copy_from_slice(&[0xFF, 0x00, 0xAA, 0x42]);
        input[4..4 + frame_len].copy_from_slice(&sink.buf[..frame_len]);

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        assert_eq!(
            frame2.read(&mut MockReader::new(&input[..4 + frame_len])),
            Ok(Status::Ok)
        );
        assert_eq!(frame2.cmd, Cmd::Verify);
    }

    #[test]
    fn read_overflow() {
        // Build a valid frame, then patch the LEN field to exceed the capacity.
        let mut f = frame(Cmd::Write, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        f.send(&mut sink).unwrap();

        // LEN is at offset 8..10 (little-endian u16). Set to capacity + 1.
        let overflow_len = (DEFAULT_PAYLOAD as u16 + 1).to_le_bytes();
        sink.buf[8] = overflow_len[0];
        sink.buf[9] = overflow_len[1];

        let mut frame2 = Frame::<DEFAULT_PAYLOAD>::default();
        assert_eq!(
            frame2.read(&mut MockReader::new(sink.written())),
            Ok(Status::PayloadOverflow)
        );
    }
}
