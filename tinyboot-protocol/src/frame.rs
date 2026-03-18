use core::mem::MaybeUninit;

use crate::crc::{CRC_INIT, crc16};
use crate::sync::Sync;
use crate::{Cmd, ReadError, Status};

/// Fixed overhead per frame: SYNC(2) + CMD(1) + LEN(1) + ADDR(2) + STATUS(1) + CRC(2).
pub const FRAME_OVERHEAD: usize = 9;

/// Derive the payload capacity from a total frame size.
pub const fn payload_size(frame_size: usize) -> usize {
    frame_size - FRAME_OVERHEAD
}

/// Wire frame: SYNC(2) + CMD(1) + LEN(1) + ADDR(2) + STATUS(1) + DATA(len) + CRC(2)
///
/// `D` is the maximum data payload size, typically derived via
/// [`payload_size`] from the transport's frame size.
///
/// Used for both requests and responses. For requests, `status` is
/// [`Status::Request`]. For responses, `cmd` and `addr` echo the request.
///
/// Single instance, reused each iteration of the protocol loop.
#[repr(C)]
pub struct Frame<const D: usize> {
    sync: Sync,
    pub cmd: Cmd,
    pub len: u8,
    pub addr: u16,
    pub status: Status,
    pub data: [u8; D],
    pub crc: [u8; 2],
}

impl<const D: usize> Default for Frame<D> {
    /// Create a default frame. Data buffer is uninitialized — `read()` or
    /// caller writes populate it before use.
    fn default() -> Self {
        let frame: MaybeUninit<Self> = MaybeUninit::uninit();
        let mut frame = unsafe { frame.assume_init() };
        frame.sync = Sync::default();
        frame.cmd = Cmd::Info;
        frame.len = 0;
        frame.addr = 0;
        frame.status = Status::Request;
        frame.crc = [0; 2];
        frame
    }
}

impl<const D: usize> Frame<D> {
    fn as_bytes(&self, offset: usize, len: usize) -> &[u8] {
        unsafe {
            let ptr = (self as *const Self as *const u8).add(offset);
            core::slice::from_raw_parts(ptr, len)
        }
    }

    fn as_bytes_mut(&mut self, offset: usize, len: usize) -> &mut [u8] {
        unsafe {
            let ptr = (self as *mut Self as *mut u8).add(offset);
            core::slice::from_raw_parts_mut(ptr, len)
        }
    }

    /// Send the frame. Two `write_all` calls: body, then CRC.
    pub fn send<W: embedded_io::Write>(&mut self, w: &mut W) -> Result<(), W::Error> {
        self.sync = Sync::valid();
        let body_len = 7 + self.len as usize;
        self.crc = crc16(CRC_INIT, self.as_bytes(0, body_len)).to_le_bytes();
        w.write_all(self.as_bytes(0, body_len))?;
        w.write_all(&self.crc)
    }

    /// Read one frame from the transport (blocking).
    ///
    /// Syncs on preamble, reads header + payload, validates CRC.
    pub fn read<R: embedded_io::Read>(&mut self, r: &mut R) -> Result<(), ReadError> {
        self.sync.read(r)?;

        // Read header fields: cmd(1) + len(1) + addr(2) + status(1) = 5 bytes at offset 2
        r.read_exact(self.as_bytes_mut(2, 5))
            .map_err(|_| ReadError::Io)?;

        if !self.cmd.is_valid() || !self.status.is_valid() {
            return Err(ReadError::InvalidFrame);
        }

        let data_len = self.len as usize;

        if data_len > D {
            return Err(ReadError::Overflow);
        }

        // Read data directly into buffer
        if data_len > 0 {
            r.read_exact(&mut self.data[..data_len])
                .map_err(|_| ReadError::Io)?;
        }

        // Read CRC directly — [u8; 2] has no alignment constraint
        r.read_exact(&mut self.crc).map_err(|_| ReadError::Io)?;

        // Validate CRC over body
        if self.crc != crc16(CRC_INIT, self.as_bytes(0, 7 + data_len)).to_le_bytes() {
            return Err(ReadError::Crc);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test frame size: 64-byte UART frame → 55 bytes payload.
    const TEST_D: usize = payload_size(64);
    type TestFrame = Frame<TEST_D>;

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

    fn frame(cmd: Cmd, status: Status, addr: u16, data: &[u8]) -> TestFrame {
        let mut f = TestFrame {
            cmd,
            status,
            addr,
            len: data.len() as u8,
            ..Default::default()
        };
        f.data[..data.len()].copy_from_slice(data);
        f
    }

    #[test]
    fn request_round_trip() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0800, &[0xDE, 0xAD]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = TestFrame::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Write);
        assert_eq!(frame2.len, 2);
        assert_eq!(frame2.addr, 0x0800);
        assert_eq!(frame2.status, Status::Request);
        assert_eq!(&frame2.data[..2], &[0xDE, 0xAD]);
        assert_eq!(frame2.crc, frame.crc);
    }

    #[test]
    fn response_round_trip() {
        let mut frame = frame(Cmd::Verify, Status::Ok, 0, &[0x12, 0x34]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        let mut frame2 = TestFrame::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Verify);
        assert_eq!(frame2.status, Status::Ok);
        assert_eq!(&frame2.data[..2], &[0x12, 0x34]);
    }

    #[test]
    fn request_no_data() {
        let mut frame = frame(Cmd::Erase, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        // Frame: SYNC(2) + CMD(1) + LEN(1) + ADDR(2) + STATUS(1) + CRC(2) = 9
        assert_eq!(sink.written().len(), 9);

        let mut frame2 = TestFrame::default();
        frame2.read(&mut MockReader::new(sink.written())).unwrap();
        assert_eq!(frame2.cmd, Cmd::Erase);
        assert_eq!(frame2.len, 0);
    }

    #[test]
    fn cmd_addr_carry_over() {
        let mut frame = frame(Cmd::Write, Status::Request, 0x0400, &[0xAB, 0xCD]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();

        // "Device" reads the request
        let mut dev = TestFrame::default();
        dev.read(&mut MockReader::new(sink.written())).unwrap();

        // Device sends response — cmd and addr carry over
        dev.status = Status::Ok;
        dev.len = 0;
        let mut resp_sink = Sink::new();
        dev.send(&mut resp_sink).unwrap();

        // Host reads response
        let mut host = TestFrame::default();
        host.read(&mut MockReader::new(resp_sink.written()))
            .unwrap();
        assert_eq!(host.cmd, Cmd::Write);
        assert_eq!(host.addr, 0x0400);
        assert_eq!(host.status, Status::Ok);
    }

    #[test]
    fn read_bad_crc() {
        let mut frame = frame(Cmd::Info, Status::Request, 0, &[]);

        let mut sink = Sink::new();
        frame.send(&mut sink).unwrap();
        sink.buf[2] ^= 0xFF; // corrupt CMD byte — caught by is_valid before CRC

        let mut frame2 = TestFrame::default();
        assert_eq!(
            frame2.read(&mut MockReader::new(sink.written())),
            Err(ReadError::InvalidFrame)
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

        let mut frame2 = TestFrame::default();
        frame2
            .read(&mut MockReader::new(&input[..4 + frame_len]))
            .unwrap();
        assert_eq!(frame2.cmd, Cmd::Verify);
    }

    #[test]
    fn read_overflow() {
        // Use a tiny frame (D=2) and try to read a payload that's too large
        let mut big_frame = frame(Cmd::Write, Status::Request, 0, &[1, 2, 3, 4]);

        let mut sink = Sink::new();
        big_frame.send(&mut sink).unwrap();

        let mut small_frame = Frame::<2>::default();
        assert_eq!(
            small_frame.read(&mut MockReader::new(sink.written())),
            Err(ReadError::Overflow)
        );
    }
}
