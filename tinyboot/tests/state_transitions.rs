//! Integration tests for boot state machine transitions.
//!
//! Tests every row of the state transition table in tinyboot/README.md.
//! Organized by operation, with each state tested per operation.

use embedded_storage::nor_flash;
use tinyboot::protocol::Dispatcher;
use tinyboot::traits::boot::{BootCtl, BootMetaStore, Platform, Storage, Transport};
use tinyboot::traits::{BootMode, BootState};
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::Frame;
use tinyboot_protocol::{Cmd, Status};

// -- Mocks --

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

    fn load_request(&mut self, cmd: Cmd, addr: u32, len: u16, data: &[u8]) {
        let mut frame = Frame::default();
        frame.cmd = cmd;
        frame.addr = addr;
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

struct MockStorage {
    data: [u8; 256],
}

impl MockStorage {
    fn new() -> Self {
        Self { data: [0xFF; 256] }
    }
}

impl Storage for MockStorage {
    fn as_slice(&self) -> &[u8] {
        &self.data
    }
    fn unlock(&mut self) {}
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

struct MockBootCtl;
impl BootCtl for MockBootCtl {
    fn is_boot_requested(&self) -> bool {
        false
    }
    fn system_reset(&mut self, _mode: BootMode) -> ! {
        panic!("mock reset")
    }
}

struct MockBootMeta {
    state: BootState,
    checksum: u16,
    trials: u8,
    app_size: u32,
}

impl MockBootMeta {
    fn new(state: BootState) -> Self {
        Self {
            state,
            checksum: 0xFFFF,
            trials: 0xFF,
            app_size: 0xFFFF_FFFF,
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

type TestPlatform = Platform<MockTransport, MockStorage, MockBootMeta, MockBootCtl>;
// 2 × MockStorage ERASE_SIZE (64)
const TEST_BUF_SIZE: usize = 128;

fn platform(state: BootState) -> TestPlatform {
    Platform::new(
        MockTransport::new(),
        MockStorage::new(),
        MockBootMeta::new(state),
        MockBootCtl,
        0xFFFF,
    )
}

fn erase_data(byte_count: u16) -> [u8; 2] {
    byte_count.to_le_bytes()
}

// =============================================================================
// Erase: state transitions
// | Idle       | → Updating   | addr/size valid | step down state byte                       |
// | Updating   | → Updating   | addr/size valid | none                                       |
// | Validating | → Updating   | addr/size valid | refresh (state=Updating, clear checksum)    |
// =============================================================================

#[test]
fn erase_from_idle_transitions_to_updating() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

#[test]
fn erase_from_updating_stays_updating() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

#[test]
fn erase_from_validating_transitions_to_updating_and_clears_checksum() {
    let mut p = platform(BootState::Validating);
    p.boot_meta.checksum = 0x1234;
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
    assert_eq!(d.platform.boot_meta.checksum, 0xFFFF);
}

#[test]
fn erase_validates_addr_alignment() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 32, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn erase_validates_byte_count_alignment() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(32));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn erase_validates_bounds() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 256, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn erase_rejects_zero_byte_count() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(0));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn erase_bulk_multiple_pages() {
    let mut p = platform(BootState::Idle);
    p.storage.data[0] = 0x42;
    p.storage.data[64] = 0x42;
    p.storage.data[128] = 0x42;
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(192));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.storage.data[0], 0xFF);
    assert_eq!(d.platform.storage.data[64], 0xFF);
    assert_eq!(d.platform.storage.data[128], 0xFF);
    // Beyond erase range untouched
    assert_eq!(d.platform.storage.data[192], 0xFF);
}

// =============================================================================
// Write: state transitions
// | Idle       | reject       |                 | not in update                              |
// | Updating   | → Updating   | addr/size valid | none                                       |
// | Validating | reject       |                 | not in update                              |
// =============================================================================

#[test]
fn write_from_idle_rejected() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Write, 0, 4, &[0xDE, 0xAD, 0xBE, 0xEF]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Unsupported);
    assert_eq!(d.platform.storage.data[0], 0xFF);
}

#[test]
fn write_from_updating_succeeds() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Write, 0, 4, &[0xDE, 0xAD, 0xBE, 0xEF]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    // Flush buffered write to storage
    d.platform.transport.load_request(Cmd::Flush, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(&d.platform.storage.data[..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

#[test]
fn write_from_validating_rejected() {
    let mut p = platform(BootState::Validating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Write, 0, 4, &[0xDE, 0xAD, 0xBE, 0xEF]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Unsupported);
}

#[test]
fn write_validates_addr_alignment() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Write, 1, 4, &[0; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn write_validates_bounds() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Write, 256, 4, &[0; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn write_at_offset() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Write, 8, 4, &[0x01, 0x02, 0x03, 0x04]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    d.platform.transport.load_request(Cmd::Flush, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(&d.platform.storage.data[8..12], &[0x01, 0x02, 0x03, 0x04]);
}

// =============================================================================
// Verify: state transitions
// | Idle       | reject       |                 | not in update                              |
// | Updating   | → Validating | CRC match       | refresh (state=Validating, write checksum) |
// | Validating | reject       |                 | already verified                           |
// =============================================================================

#[test]
fn verify_from_idle_rejected() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Verify, 4, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Unsupported);
}

#[test]
fn verify_from_updating_transitions_to_validating() {
    let mut p = platform(BootState::Updating);
    let fw = [0x01, 0x02, 0x03, 0x04];
    p.storage.data[..4].copy_from_slice(&fw);
    let app_size = fw.len() as u32;
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Verify, app_size, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Validating);
    assert_eq!(d.frame.len, 2);
    let expected = crc16(CRC_INIT, &fw);
    assert_eq!(unsafe { d.frame.data.verify }.crc, expected);
    assert_eq!(d.platform.boot_meta.checksum, expected);
    assert_eq!(d.platform.boot_meta.app_size, app_size);
}

#[test]
fn verify_from_validating_rejected() {
    let mut p = platform(BootState::Validating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Verify, 4, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Unsupported);
}

#[test]
fn verify_rejects_zero_app_size() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Verify, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

#[test]
fn verify_rejects_app_size_exceeding_capacity() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    // capacity is 256, send 257
    d.platform.transport.load_request(Cmd::Verify, 257, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

// =============================================================================
// Info: works in all states (no state change)
// =============================================================================

#[test]
fn info_reports_geometry() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Info, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.frame.len, 12);
    let info = unsafe { d.frame.data.info };
    assert_eq!({ info.capacity }, 256);
    assert_eq!({ info.erase_size }, 64);
    assert_eq!({ info.mode }, 0);
}

// =============================================================================
// Multi-step sequences
// =============================================================================

#[test]
fn full_update_cycle() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    // Erase: Idle → Updating
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);

    // Write: Updating → Updating
    let fw = [0x01, 0x02, 0x03, 0x04];
    d.platform.transport.load_request(Cmd::Write, 0, 4, &fw);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);

    // Flush buffered writes
    d.platform.transport.load_request(Cmd::Flush, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    // Verify: Updating → Validating (app_size=4)
    d.platform
        .transport
        .load_request(Cmd::Verify, fw.len() as u32, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Validating);
    assert_ne!(d.platform.boot_meta.checksum, 0xFFFF);
    assert_eq!(d.platform.boot_meta.app_size, fw.len() as u32);
}

#[test]
fn reflash_from_validating() {
    let mut p = platform(BootState::Validating);
    p.boot_meta.checksum = 0x1234;
    p.boot_meta.app_size = 4;
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    // Erase: Validating → Updating (reflash, clears checksum + app_size)
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
    assert_eq!(d.platform.boot_meta.checksum, 0xFFFF);
    assert_eq!(d.platform.boot_meta.app_size, 0xFFFF_FFFF);

    // Write succeeds
    let fw = [0xAB, 0xCD, 0xEF, 0x01];
    d.platform.transport.load_request(Cmd::Write, 0, 4, &fw);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    // Flush buffered writes
    d.platform.transport.load_request(Cmd::Flush, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    // Verify: Updating → Validating
    d.platform
        .transport
        .load_request(Cmd::Verify, fw.len() as u32, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Validating);
    assert_eq!(d.platform.boot_meta.app_size, fw.len() as u32);
}

// =============================================================================
// Flush
// =============================================================================

#[test]
fn flush_commits_buffered_write() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Write, 0, 2, &[0xAA, 0xBB]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    // Data is buffered (2 < WRITE_SIZE=4), not yet in storage
    assert_eq!(d.platform.storage.data[0], 0xFF);

    d.platform.transport.load_request(Cmd::Flush, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(&d.platform.storage.data[..2], &[0xAA, 0xBB]);
}

#[test]
fn flush_empty_is_ok() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Flush, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
}

#[test]
fn write_non_sequential_without_flush_rejected() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    // First write at addr 0
    d.platform.transport.load_request(Cmd::Write, 0, 4, &[0; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    // Non-sequential write without Flush
    d.platform
        .transport
        .load_request(Cmd::Write, 128, 4, &[0; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}
