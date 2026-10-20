//! Mocks and helpers shared by the integration tests.

#![allow(dead_code)]

use embedded_storage::nor_flash;
use tinyboot_core::Platform;
use tinyboot_core::protocol::Dispatcher;
use tinyboot_core::traits::{BootCtl, BootMetaStore, BootState, RunMode, Storage, Transport};
use tinyboot_protocol::frame::{DEFAULT_PAYLOAD, Flags, Frame};
use tinyboot_protocol::{Cmd, Status, WriteFlags};

// -- Mocks --

pub struct MockTransport {
    pub rx_buf: [u8; 512],
    pub rx_len: usize,
    pub rx_pos: usize,
    pub tx_buf: [u8; 512],
    pub tx_pos: usize,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            rx_buf: [0; 512],
            rx_len: 0,
            rx_pos: 0,
            tx_buf: [0; 512],
            tx_pos: 0,
        }
    }

    pub fn load_request(&mut self, cmd: Cmd, addr: u32, len: u16, data: &[u8]) {
        self.load_request_flags(cmd, addr, Flags { raw: 0 }, len, data);
    }

    pub fn load_request_flags(&mut self, cmd: Cmd, addr: u32, flags: Flags, len: u16, data: &[u8]) {
        self.load_request_flags_n::<DEFAULT_PAYLOAD>(cmd, addr, flags, len, data);
    }

    /// As [`load_request_flags`](Self::load_request_flags) through an
    /// `N`-byte host frame.
    pub fn load_request_flags_n<const N: usize>(
        &mut self,
        cmd: Cmd,
        addr: u32,
        flags: Flags,
        len: u16,
        data: &[u8],
    ) {
        let mut frame = Frame::<N>::default();
        frame.cmd = cmd;
        frame.set_addr(addr);
        frame.flags = flags;
        frame.len = len;
        frame.status = Status::Request;
        unsafe { frame.data.raw[..data.len()].copy_from_slice(data) };
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

impl Transport for MockTransport {}

// Word-aligned like flash, for the bootloader's word load of an image key.
#[repr(C, align(4))]
pub struct MockStorage {
    pub data: [u8; 256],
}

impl MockStorage {
    pub fn new() -> Self {
        Self { data: [0xFF; 256] }
    }
}

impl Storage for MockStorage {
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

pub struct MockBootCtl;
impl BootCtl for MockBootCtl {
    fn run_mode(&self) -> RunMode {
        RunMode::HandOff
    }
    fn set_run_mode(&mut self, _mode: RunMode) {}
    fn reset(&mut self) -> ! {
        panic!("mock reset")
    }
    fn hand_off(&mut self) -> ! {
        panic!("mock hand_off")
    }
}

pub struct MockBootMeta {
    pub state: BootState,
    pub checksum: u16,
    pub trials: u8,
    pub app_size: u32,
    pub baud: u8,
    pub baud_writes: u32,
}

impl MockBootMeta {
    pub fn new(state: BootState) -> Self {
        Self {
            state,
            checksum: 0xFFFF,
            trials: 0xFF,
            app_size: 0xFFFF_FFFF,
            baud: 0xFF,
            baud_writes: 0,
        }
    }
}

impl BootMetaStore for MockBootMeta {
    type Error = ();
    fn boot_state(&self) -> BootState {
        self.state
    }
    fn has_trials(&self) -> bool {
        self.trials != 0
    }
    fn app_checksum(&self) -> u16 {
        self.checksum
    }
    fn app_size(&self) -> u32 {
        self.app_size
    }
    fn baud(&self) -> u8 {
        self.baud
    }
    fn set_baud(&mut self, baud: u8) -> Result<(), ()> {
        self.baud = baud;
        self.baud_writes += 1;
        Ok(())
    }
    fn advance(&mut self) -> Result<BootState, ()> {
        let next = self.state as u8;
        let next = next & (next >> 1);
        self.state = BootState::from_u8(next);
        Ok(self.state)
    }
    fn consume_trial(&mut self) -> Result<(), ()> {
        self.trials = self.trials & (self.trials >> 1);
        Ok(())
    }
    fn refresh(&mut self, checksum: u16, state: BootState, app_size: u32) -> Result<(), ()> {
        self.state = state;
        self.checksum = checksum;
        self.trials = 0xFF;
        self.app_size = app_size;
        Ok(())
    }
}

pub type TestPlatform = Platform<MockTransport, MockStorage, MockBootMeta, MockBootCtl>;
pub const TEST_BUF_SIZE: usize = 128;

pub fn platform(state: BootState) -> TestPlatform {
    Platform::new(
        MockTransport::new(),
        MockStorage::new(),
        MockBootMeta::new(state),
        MockBootCtl,
    )
}

pub fn erase_data(byte_count: u16) -> [u8; 2] {
    byte_count.to_le_bytes()
}

pub fn flush_flags() -> Flags {
    Flags {
        write: WriteFlags::FLUSH,
    }
}

/// Store an image of `len` bytes ending in a tail with `key`. `len` must
/// be 2 past a word, as `tb-app.x` lays images out.
pub fn load_tagged_image(storage: &mut MockStorage, len: usize, key: u32) {
    storage.data[..len].fill(0x5A);
    storage.data[len - 6..len - 2].copy_from_slice(&key.to_le_bytes());
    storage.data[len - 2..len].copy_from_slice(&0x0841u16.to_le_bytes());
}

/// Dispatch `Verify` of `app_size` bytes from Updating.
pub fn verify(p: &mut TestPlatform, app_size: u32) -> Status {
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(p);
    d.platform
        .transport
        .load_request(Cmd::Verify, app_size, 0, &[]);
    d.dispatch().unwrap();
    d.frame.status
}
