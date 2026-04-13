use core::mem::MaybeUninit;

use crate::crc::{CRC_INIT, crc16};
use crate::sync::Sync;
use crate::{Cmd, ReadError, Status};
use tinyboot_macros::tb_assert;

/// Maximum data payload size per frame.
pub const MAX_PAYLOAD: usize = 64;

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
    pub mode: u16,
}

/// Typed Erase request data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EraseData {
    /// Number of bytes to erase (must be aligned to erase size).
    pub byte_count: u16,
}

/// Typed Verify response data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VerifyData {
    /// CRC16 of the app region.
    pub crc: u16,
}

/// Union-typed data payload.
///
/// Provides zero-cost typed access to frame data. Reading fields is unsafe
/// because the caller must know which variant is active.
#[repr(C)]
pub union Data {
    /// Raw byte access.
    pub raw: [u8; MAX_PAYLOAD],
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
/// Single instance, reused each iteration of the protocol loop.
#[repr(C)]
pub struct Frame {
    sync: Sync,
    /// Command code.
    pub cmd: Cmd,
    /// Response status (always [`Status::Request`] for requests).
    pub status: Status,
    /// Flash address (for Write/Erase) or mode selector (for Reset).
    pub addr: u32,
    /// Data payload length in bytes (0..64).
    pub len: u16,
    /// Payload data (union-typed).
    pub data: Data,
    /// CRC16 over the frame body (little-endian).
    pub crc: [u8; 2],
}

impl Default for Frame {
    /// Create a default frame. Data buffer is uninitialized — `read()` or
    /// caller writes populate it before use.
    fn default() -> Self {
        let frame: MaybeUninit<Self> = MaybeUninit::uninit();
        let mut frame = unsafe { frame.assume_init() };
        frame.sync = Sync::default();
        frame.cmd = Cmd::Info;
        frame.status = Status::Request;
        frame.addr = 0;
        frame.len = 0;
        frame.crc = [0; 2];
        frame
    }
}

impl Frame {
    fn as_bytes(&self, offset: usize, len: usize) -> &[u8] {
        tb_assert!(offset + len <= core::mem::size_of::<Self>());
        unsafe {
            let ptr = (self as *const Self as *const u8).add(offset);
            core::slice::from_raw_parts(ptr, len)
        }
    }

    fn as_bytes_mut(&mut self, offset: usize, len: usize) -> &mut [u8] {
        tb_assert!(offset + len <= core::mem::size_of::<Self>());
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
        // (inside `data` when len < MAX_PAYLOAD, or at `crc` field when len == MAX_PAYLOAD).
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

        if data_len > MAX_PAYLOAD {
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

        if data_len > MAX_PAYLOAD {
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
            addr,
            len: data.len() as u16,
            ..Default::default()
        };
        unsafe { f.data.raw[..data.len()].copy_from_slice(data) };
        f
    }

    #[test]
    fn request_round_trip() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0800, &[0xDE, 0xAD]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = Frame::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Write);
        assert_eq!(frame2.len, 2);
        assert_eq!(frame2.addr, 0x0800);
        assert_eq!(frame2.status, Status::Request);
        assert_eq!(unsafe { &frame2.data.raw[..2] }, &[0xDE, 0xAD]);
    }

    #[test]
    fn response_round_trip() {
        let mut frame = frame(Cmd::Verify, Status::Ok, 0, &[0x12, 0x34]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = Frame::default();
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

        let mut frame2 = Frame::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Erase);
        assert_eq!(frame2.len, 0);
    }

    #[test]
    fn large_addr_round_trip() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0001_0800, &[0xAB]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = Frame::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.addr, 0x0001_0800);
    }

    #[test]
    fn cmd_addr_carry_over() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0400, &[0xAB, 0xCD]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        // "Device" reads the request
        let mut dev = Frame::default();
        dev.read(&mut MockReader::new(sink.written())).unwrap();

        // Device sends response — cmd and addr carry over
        dev.status = Status::Ok;
        dev.len = 0;
        let mut resp_sink = Sink::new();
        dev.send(&mut resp_sink).unwrap();

        // Host reads response
        let mut host = Frame::default();
        host.read(&mut MockReader::new(resp_sink.written()))
            .unwrap();
        assert_eq!(host.cmd, Cmd::Write);
        assert_eq!(host.addr, 0x0400);
        assert_eq!(host.status, Status::Ok);
    }

    #[test]
    fn read_bad_cmd() {
        let mut frame = frame(Cmd::Info, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();
        sink.buf[2] ^= 0xFF; // corrupt CMD byte

        let mut frame2 = Frame::default();
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

        let mut frame2 = Frame::default();
        assert_eq!(
            frame2.read(&mut MockReader::new(&input[..4 + frame_len])),
            Ok(Status::Ok)
        );
        assert_eq!(frame2.cmd, Cmd::Verify);
    }

    #[test]
    fn read_overflow() {
        // Build a valid frame, then patch the LEN field to exceed MAX_PAYLOAD.
        let mut f = frame(Cmd::Write, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        f.send(&mut sink).unwrap();

        // LEN is at offset 8..10 (little-endian u16). Set to MAX_PAYLOAD + 1.
        let overflow_len = (MAX_PAYLOAD as u16 + 1).to_le_bytes();
        sink.buf[8] = overflow_len[0];
        sink.buf[9] = overflow_len[1];

        let mut frame2 = Frame::default();
        assert_eq!(
            frame2.read(&mut MockReader::new(sink.written())),
            Ok(Status::PayloadOverflow)
        );
    }
}
