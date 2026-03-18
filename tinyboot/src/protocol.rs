use crate::traits::{BootCtl, BootMetaStore, Platform, Storage, Transport};
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{Frame, InfoData, VerifyData};
use tinyboot_protocol::{Cmd, ReadError, Status};

/// Protocol dispatcher. Borrows the platform, owns the frame.
pub struct Dispatcher<'a, const D: usize, T: Transport<D>, S: Storage, B: BootMetaStore, C: BootCtl>
{
    pub platform: &'a mut Platform<D, T, S, B, C>,
    pub frame: Frame<D>,
}

impl<'a, const D: usize, T: Transport<D>, S: Storage, B: BootMetaStore, C: BootCtl>
    Dispatcher<'a, D, T, S, B, C>
{
    pub fn new(platform: &'a mut Platform<D, T, S, B, C>) -> Self {
        Self {
            platform,
            frame: Frame::default(),
        }
    }

    /// Read a frame, dispatch the command, and send the response.
    #[inline(never)]
    pub fn dispatch(&mut self) -> Result<(), ReadError> {
        self.frame.read(&mut self.platform.transport)?;

        let data_len = self.frame.len as usize;
        let capacity = self.platform.storage.capacity() as u32;
        let erase_size = S::ERASE_SIZE as u32;
        let write_size = S::WRITE_SIZE as u32;
        self.frame.len = 0;
        self.frame.status = Status::Ok;

        match self.frame.cmd {
            Cmd::Info => {
                self.frame.len = 8;
                self.frame.data.info = InfoData {
                    capacity,
                    payload_size: D as u16,
                    erase_size: erase_size as u16,
                };
            }
            Cmd::Erase => {
                let addr = self.frame.addr;
                if !addr.is_multiple_of(erase_size) || addr + erase_size > capacity {
                    self.frame.status = Status::AddrOutOfBounds;
                } else if self
                    .platform
                    .storage
                    .erase(addr, addr + erase_size)
                    .is_err()
                {
                    self.frame.status = Status::WriteError;
                }
            }
            Cmd::Write => {
                let addr = self.frame.addr;

                if addr >= capacity
                    || addr + data_len as u32 > capacity
                    || !addr.is_multiple_of(write_size)
                {
                    self.frame.status = Status::AddrOutOfBounds;
                } else if self
                    .platform
                    .storage
                    .write(addr, unsafe { &self.frame.data.raw[..data_len] })
                    .is_err()
                {
                    self.frame.status = Status::WriteError;
                }
            }
            Cmd::Verify => {
                let crc = crc16(CRC_INIT, self.platform.storage.as_slice());
                self.frame.len = 2;
                self.frame.data.verify = VerifyData { crc };
                #[cfg(feature = "trial-boot")]
                if self.platform.boot_meta.advance().is_err() {
                    self.frame.status = Status::WriteError;
                }
            }
            Cmd::Reset => {
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
    use tinyboot_protocol::frame::payload_size;

    const TEST_D: usize = payload_size(64);

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
        fn load_request(&mut self, cmd: Cmd, addr: u32, len: u16, data: &[u8]) {
            let mut frame = Frame::<TEST_D>::default();
            frame.cmd = cmd;
            frame.addr = addr;
            frame.len = len;
            frame.status = Status::Request;
            unsafe { frame.data.raw[..data.len()].copy_from_slice(data) };

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

    impl Transport<TEST_D> for MockTransport {}

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
        const ERASE_SIZE: usize = 64;

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
            panic!("mock reset")
        }
        fn boot_app(&mut self) -> ! {
            panic!("mock boot_app")
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

    fn make_platform(
        storage: MockStorage,
    ) -> Platform<TEST_D, MockTransport, MockStorage, MockBootMeta, MockBootCtl> {
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
        assert_eq!(d.frame.len, 8);
        let info = unsafe { d.frame.data.info };
        assert_eq!({ info.capacity }, 256);
        assert_eq!({ info.payload_size }, TEST_D as u16);
        assert_eq!({ info.erase_size }, 64);
    }

    #[test]
    fn erase_clears_page() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.storage.data[0] = 0x42;
        d.platform.storage.data[64] = 0x42;
        d.platform.transport.load_request(Cmd::Erase, 0, 0, &[]);

        d.dispatch().unwrap();

        assert_eq!(d.platform.storage.data[0], 0xFF);
        // Second page untouched
        assert_eq!(d.platform.storage.data[64], 0x42);
        assert_eq!(d.frame.status, Status::Ok);
    }

    #[test]
    fn erase_second_page() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.storage.data[0] = 0x42;
        d.platform.storage.data[64] = 0x42;
        d.platform.transport.load_request(Cmd::Erase, 64, 0, &[]);

        d.dispatch().unwrap();

        // First page untouched
        assert_eq!(d.platform.storage.data[0], 0x42);
        assert_eq!(d.platform.storage.data[64], 0xFF);
        assert_eq!(d.frame.status, Status::Ok);
    }

    #[test]
    fn erase_out_of_bounds() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.transport.load_request(Cmd::Erase, 256, 0, &[]);

        d.dispatch().unwrap();
        assert_eq!(d.frame.status, Status::AddrOutOfBounds);
    }

    #[test]
    fn erase_unaligned() {
        let mut p = make_platform(MockStorage::new());
        let mut d = Dispatcher::new(&mut p);
        d.platform.transport.load_request(Cmd::Erase, 32, 0, &[]);

        d.dispatch().unwrap();
        assert_eq!(d.frame.status, Status::AddrOutOfBounds);
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
        assert_eq!(unsafe { d.frame.data.verify }.crc, expected);
    }
}
