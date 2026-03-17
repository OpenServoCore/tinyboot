use crate::traits::{BootCtl, BootMetaStore, Platform, Storage, Transport};
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::Frame;
use tinyboot_protocol::{Cmd, ReadError, Status};

/// Protocol dispatcher. Borrows the platform, owns the frame.
pub struct Dispatcher<'a, T: Transport, S: Storage, B: BootMetaStore, C: BootCtl> {
    pub platform: &'a mut Platform<T, S, B, C>,
    pub frame: Frame,
}

impl<'a, T: Transport, S: Storage, B: BootMetaStore, C: BootCtl> Dispatcher<'a, T, S, B, C> {
    pub fn new(platform: &'a mut Platform<T, S, B, C>) -> Self {
        Self {
            platform,
            frame: Frame::new(),
        }
    }

    /// Read a frame, dispatch the command, and send the response.
    #[inline(never)]
    pub fn dispatch(&mut self) -> Result<(), ReadError> {
        self.frame.read(&mut self.platform.transport)?;

        let data_len = self.frame.len as usize;
        let capacity = self.platform.storage.capacity();
        self.frame.len = 0;
        self.frame.status = Status::Ok;

        match self.frame.cmd {
            Cmd::Info => {
                let ws = (S::WRITE_SIZE as u16).to_le_bytes();
                let cap = (capacity as u16).to_le_bytes();
                self.frame.len = 6;
                self.frame.data[0] = ws[0];
                self.frame.data[1] = ws[1];
                self.frame.data[2] = cap[0];
                self.frame.data[3] = cap[1];
                self.frame.data[4] = ws[0];
                self.frame.data[5] = ws[1];
            }
            Cmd::Erase => {
                if self.platform.storage.erase(0, capacity as u32).is_err() {
                    self.frame.status = Status::Error;
                }
            }
            Cmd::Write => {
                let addr = self.frame.addr as u32;

                if addr >= capacity as u32
                    || addr + data_len as u32 > capacity as u32
                    || addr as usize % S::WRITE_SIZE != 0
                {
                    self.frame.status = Status::AddrOutOfBounds;
                } else if self
                    .platform
                    .storage
                    .write(addr, &self.frame.data[..data_len])
                    .is_err()
                {
                    self.frame.status = Status::Error;
                }
            }
            Cmd::Verify => {
                let crc = crc16(CRC_INIT, self.platform.storage.as_slice());
                let crc_bytes = crc.to_le_bytes();
                self.frame.len = 2;
                self.frame.data[0] = crc_bytes[0];
                self.frame.data[1] = crc_bytes[1];
            }
            Cmd::Reset => {
                let _ = self.platform.boot_meta.advance();
                let _ = self.frame.send(&mut self.platform.transport);
                self.platform.ctl.system_reset();
            }
        }

        self.frame
            .send(&mut self.platform.transport)
            .map_err(|_| ReadError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_storage::nor_flash;

    // -- Mock transport (read feeds rx_buf, write captures to tx_buf) --

    struct MockTransport {
        rx_buf: [u8; 512],
        rx_len: usize,
        rx_pos: usize,
        tx_buf: [u8; 512],
        tx_pos: usize,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                rx_buf: [0; 512],
                rx_len: 0,
                rx_pos: 0,
                tx_buf: [0; 512],
                tx_pos: 0,
            }
        }

        /// Load a request frame into the rx buffer by sending it through Frame.
        fn load_request(&mut self, cmd: Cmd, addr: u16, len: u8, data: &[u8]) {
            let mut frame = Frame::new();
            frame.cmd = cmd;
            frame.addr = addr;
            frame.len = len;
            frame.status = Status::Request;
            frame.data[..data.len()].copy_from_slice(data);

            // Send into a temp buffer, then copy to rx_buf
            let mut tmp = MockTransport::new();
            frame.send(&mut tmp).unwrap();
            self.rx_buf[..tmp.tx_pos].copy_from_slice(&tmp.tx_buf[..tmp.tx_pos]);
            self.rx_len = tmp.tx_pos;
            self.rx_pos = 0;
        }
    }

    impl embedded_io::ErrorType for MockTransport {
        type Error = core::convert::Infallible;
    }

    impl embedded_io::Read for MockTransport {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let n = buf.len().min(self.rx_len - self.rx_pos);
            buf[..n].copy_from_slice(&self.rx_buf[self.rx_pos..self.rx_pos + n]);
            self.rx_pos += n;
            Ok(n)
        }
    }

    impl embedded_io::Write for MockTransport {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            let n = buf.len().min(self.tx_buf.len() - self.tx_pos);
            self.tx_buf[self.tx_pos..self.tx_pos + n].copy_from_slice(&buf[..n]);
            self.tx_pos += n;
            Ok(n)
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    // -- Mock storage --

    struct MockStorage {
        data: [u8; 256],
    }

    impl MockStorage {
        fn new() -> Self {
            Self { data: [0xFF; 256] }
        }
    }

    impl crate::traits::Storage for MockStorage {
        fn as_slice(&self) -> &[u8] {
            &self.data
        }
    }

    impl nor_flash::ErrorType for MockStorage {
        type Error = nor_flash::NorFlashErrorKind;
    }

    impl nor_flash::ReadNorFlash for MockStorage {
        const READ_SIZE: usize = 1;

        fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
            let start = offset as usize;
            let end = start + bytes.len();
            if end > self.data.len() {
                return Err(nor_flash::NorFlashErrorKind::OutOfBounds);
            }
            bytes.copy_from_slice(&self.data[start..end]);
            Ok(())
        }

        fn capacity(&self) -> usize {
            self.data.len()
        }
    }

    impl nor_flash::NorFlash for MockStorage {
        const WRITE_SIZE: usize = 4;
        const ERASE_SIZE: usize = 256;

        fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
            self.data[from as usize..to as usize].fill(0xFF);
            Ok(())
        }

        fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
            let start = offset as usize;
            let end = start + bytes.len();
            if end > self.data.len() {
                return Err(nor_flash::NorFlashErrorKind::OutOfBounds);
            }
            self.data[start..end].copy_from_slice(bytes);
            Ok(())
        }
    }

    // -- Test helper: mock BootCtl and BootMetaStore for Platform --

    struct MockBootCtl;
    impl BootCtl for MockBootCtl {
        fn is_boot_requested(&self) -> bool {
            false
        }
        fn clear_boot_request(&mut self) {}
        fn system_reset(&mut self) -> ! {
            loop {}
        }
    }

    struct MockBootMeta;
    impl BootMetaStore for MockBootMeta {
        type Error = ();
        fn read(&self) -> crate::traits::BootMeta {
            crate::traits::BootMeta {
                state: 0xFFFF,
                trials: 0xFFFF,
                app_checksum: 0,
                app_size: 0,
            }
        }
        fn advance(&mut self) -> Result<crate::traits::BootState, ()> {
            Ok(crate::traits::BootState::Idle)
        }
        fn consume_trial(&mut self) -> Result<(), ()> {
            Ok(())
        }
    }

    type TestDispatcher<'a> = Dispatcher<'a, MockTransport, MockStorage, MockBootMeta, MockBootCtl>;

    fn make_platform(
        storage: MockStorage,
    ) -> Platform<MockTransport, MockStorage, MockBootMeta, MockBootCtl> {
        Platform::new(MockTransport::new(), storage, MockBootMeta, MockBootCtl)
    }

    // -- Tests --

    #[test]
    fn info_reports_geometry() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.transport.load_request(Cmd::Info, 0, 0, &[]);

        d.dispatch().unwrap();

        assert_eq!(d.frame.status, Status::Ok);
        assert_eq!(d.frame.len, 6);
        assert_eq!(u16::from_le_bytes([d.frame.data[0], d.frame.data[1]]), 4);
        assert_eq!(u16::from_le_bytes([d.frame.data[2], d.frame.data[3]]), 256);
    }

    #[test]
    fn erase_clears_storage() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.storage.data[0] = 0x42;
        d.platform.transport.load_request(Cmd::Erase, 0, 0, &[]);

        d.dispatch().unwrap();

        assert_eq!(d.platform.storage.data[0], 0xFF);
        assert_eq!(d.frame.status, Status::Ok);
    }

    #[test]
    fn write_stores_data() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform
            .transport
            .load_request(Cmd::Write, 0, 4, &[0xDE, 0xAD, 0xBE, 0xEF]);

        d.dispatch().unwrap();

        assert_eq!(&d.platform.storage.data[..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(d.frame.status, Status::Ok);
    }

    #[test]
    fn write_at_offset() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform
            .transport
            .load_request(Cmd::Write, 8, 4, &[0x01, 0x02, 0x03, 0x04]);

        d.dispatch().unwrap();

        assert_eq!(&d.platform.storage.data[8..12], &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn write_out_of_bounds() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform
            .transport
            .load_request(Cmd::Write, 256, 4, &[0; 4]);

        d.dispatch().unwrap();
        assert_eq!(d.frame.status, Status::AddrOutOfBounds);
    }

    #[test]
    fn write_unaligned_addr() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.transport.load_request(Cmd::Write, 1, 4, &[0; 4]);

        d.dispatch().unwrap();
        assert_eq!(d.frame.status, Status::AddrOutOfBounds);
    }

    #[test]
    fn verify_computes_crc() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.storage.data[..4].copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        d.platform.transport.load_request(Cmd::Verify, 0, 0, &[]);

        d.dispatch().unwrap();

        assert_eq!(d.frame.status, Status::Ok);
        assert_eq!(d.frame.len, 2);
        let expected = crc16(CRC_INIT, &d.platform.storage.data);
        assert_eq!(d.frame.data[0], expected as u8);
        assert_eq!(d.frame.data[1], (expected >> 8) as u8);
    }
}
