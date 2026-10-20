//! Integration tests for boot state machine transitions.
//!
//! Tests every row of the state transition table in tinyboot/README.md.
//! Organized by operation, with each state tested per operation.

mod common;

use common::*;
use tinyboot_core::app::{App, AppConfig};
use tinyboot_core::protocol::Dispatcher;
use tinyboot_core::traits::BootState;
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::DEFAULT_PAYLOAD;
use tinyboot_protocol::frame::Flags;
use tinyboot_protocol::{BaudRate, COMPAT_UNSET, Cmd, ResetFlags, Status};

// This bootloader's image tail: no compatibility key.
tinyboot_core::boot_version!();

// =============================================================================
// Erase
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
    assert_eq!(d.platform.storage.data[192], 0xFF);
}

// =============================================================================
// Write
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
    d.platform.transport.load_request_flags(
        Cmd::Write,
        0,
        flush_flags(),
        4,
        &[0xDE, 0xAD, 0xBE, 0xEF],
    );
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
    d.platform.transport.load_request_flags(
        Cmd::Write,
        4,
        flush_flags(),
        4,
        &[0x01, 0x02, 0x03, 0x04],
    );
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(&d.platform.storage.data[4..8], &[0x01, 0x02, 0x03, 0x04]);
}

// =============================================================================
// Verify
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
fn unset_key_skips_the_check() {
    let mut p = platform(BootState::Updating);
    load_tagged_image(&mut p.storage, 10, tinyboot_core::compat_key("board-b"));
    assert_eq!(verify(&mut p, 10), Status::Ok);
    assert_eq!(p.boot_meta.state, BootState::Validating);
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
    d.platform.transport.load_request(Cmd::Verify, 257, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

// =============================================================================
// Info
// =============================================================================

#[test]
fn info_reports_geometry() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request(Cmd::Info, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.frame.len, 18);
    let info = unsafe { d.frame.data.info };
    assert_eq!({ info.capacity }, 256);
    assert_eq!({ info.erase_size }, 64);
    assert_eq!({ info.mode }, 0);
    assert_eq!({ info.boot_baud }, 0xFF);
    assert_eq!({ info.compat }, COMPAT_UNSET);
    assert_eq!({ info.max_payload }, DEFAULT_PAYLOAD as u16);
}

#[test]
fn info_reports_stored_baud_and_payload_capacity() {
    let mut p = platform(BootState::Idle);
    p.boot_meta.baud = BaudRate::B1000000 as u8;
    let mut d = Dispatcher::<_, _, _, _, 4, 240>::new(&mut p);
    d.platform.transport.load_request(Cmd::Info, 0, 0, &[]);
    d.dispatch().unwrap();
    let info = unsafe { d.frame.data.info };
    assert_eq!({ info.boot_baud }, BaudRate::B1000000 as u8);
    assert_eq!({ info.max_payload }, 240);
}

#[test]
fn write_accepts_a_full_large_payload() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, 4, 240>::new(&mut p);
    let data = [0xA5; 240];
    d.platform
        .transport
        .load_request_flags_n::<240>(Cmd::Write, 0, flush_flags(), 240, &data);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert!(d.platform.storage.data[..240].iter().all(|&b| b == 0xA5));
}

// =============================================================================
// Reset
// =============================================================================

fn reset_flags(flags: ResetFlags) -> Flags {
    Flags { reset: flags }
}

/// Dispatch a request that ends in the mock's panicking reset.
fn dispatch_reset(
    d: &mut Dispatcher<'_, MockTransport, MockStorage, MockBootMeta, MockBootCtl, TEST_BUF_SIZE>,
) {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = d.dispatch();
    }));
    assert!(r.is_err(), "reset must not return");
}

#[test]
fn reset_with_baud_flag_stores_baud_and_replies() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request_flags(
        Cmd::Reset,
        BaudRate::B1000000 as u32,
        reset_flags(ResetFlags::BAUD | ResetFlags::BOOTLOADER),
        0,
        &[],
    );
    dispatch_reset(&mut d);
    assert_eq!(d.platform.boot_meta.baud, BaudRate::B1000000 as u8);
    assert_eq!(d.frame.status, Status::Ok);
    assert_ne!(d.platform.transport.tx_pos, 0);
}

#[test]
fn reset_without_baud_flag_keeps_baud() {
    let mut p = platform(BootState::Idle);
    p.boot_meta.baud = BaudRate::B500000 as u8;
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request_flags(
        Cmd::Reset,
        BaudRate::B9600 as u32,
        reset_flags(ResetFlags::BOOTLOADER),
        0,
        &[],
    );
    dispatch_reset(&mut d);
    assert_eq!(d.platform.boot_meta.baud, BaudRate::B500000 as u8);
    assert_eq!(d.platform.boot_meta.baud_writes, 0);
}

#[test]
fn reset_to_app_refused_while_updating() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request_flags(
        Cmd::Reset,
        BaudRate::B9600 as u32,
        reset_flags(ResetFlags::BAUD),
        0,
        &[],
    );
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Unsupported);
    assert_eq!(d.platform.boot_meta.baud_writes, 0);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
}

#[test]
fn reset_to_bootloader_allowed_while_updating() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request_flags(
        Cmd::Reset,
        0,
        reset_flags(ResetFlags::BOOTLOADER),
        0,
        &[],
    );
    dispatch_reset(&mut d);
    assert_eq!(d.frame.status, Status::Ok);
}

#[test]
fn reset_to_app_allowed_while_validating() {
    let mut p = platform(BootState::Validating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform.transport.load_request_flags(
        Cmd::Reset,
        0,
        reset_flags(ResetFlags::empty()),
        0,
        &[],
    );
    dispatch_reset(&mut d);
    assert_eq!(d.frame.status, Status::Ok);
}

#[test]
fn erase_restarts_an_update() {
    let mut p = platform(BootState::Updating);
    p.storage.data[..4].copy_from_slice(&[1, 2, 3, 4]);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(256));
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
    assert!(d.platform.storage.data.iter().all(|&b| b == 0xFF));
}

// =============================================================================
// App
// =============================================================================

fn app() -> App<MockBootCtl, MockBootMeta> {
    App::new(
        AppConfig {
            capacity: 256,
            erase_size: 64,
            boot_version: 0xFFFF,
            boot_compat: COMPAT_UNSET,
            app_version: 0xFFFF,
        },
        MockBootCtl,
        MockBootMeta::new(BootState::Idle),
    )
}

#[test]
fn app_baud_defaults_to_none() {
    assert_eq!(app().baud(), None);
}

#[test]
fn app_set_baud_round_trips() {
    let mut a = app();
    a.set_baud(BaudRate::B1000000);
    assert_eq!(a.baud(), Some(BaudRate::B1000000));
}

#[test]
fn app_reset_with_baud_flag_stores_baud() {
    let mut a = app();
    let mut rx = MockTransport::new();
    rx.load_request_flags(
        Cmd::Reset,
        BaudRate::B2000000 as u32,
        reset_flags(ResetFlags::BAUD),
        0,
        &[],
    );
    let mut tx = MockTransport::new();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a.poll(&mut rx, &mut tx)));
    assert!(r.is_err(), "reset must not return");
    assert_eq!(a.baud(), Some(BaudRate::B2000000));
}

// =============================================================================
// Multi-step sequences
// =============================================================================

#[test]
fn full_update_cycle() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);

    let fw = [0x01, 0x02, 0x03, 0x04];
    d.platform
        .transport
        .load_request_flags(Cmd::Write, 0, flush_flags(), 4, &fw);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

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

    d.platform
        .transport
        .load_request(Cmd::Erase, 0, 2, &erase_data(64));
    d.dispatch().unwrap();
    assert_eq!(d.platform.boot_meta.state, BootState::Updating);
    assert_eq!(d.platform.boot_meta.checksum, 0xFFFF);

    let fw = [0xAB, 0xCD, 0xEF, 0x01];
    d.platform
        .transport
        .load_request_flags(Cmd::Write, 0, flush_flags(), 4, &fw);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    d.platform
        .transport
        .load_request(Cmd::Verify, fw.len() as u32, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.boot_meta.state, BootState::Validating);
}

// =============================================================================
// Write FLUSH flag
// =============================================================================

#[test]
fn flush_flag_commits_buffered_write() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    d.platform
        .transport
        .load_request(Cmd::Write, 0, 2, &[0xAA, 0xBB]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(d.platform.storage.data[0], 0xFF); // still buffered

    d.platform
        .transport
        .load_request_flags(Cmd::Write, 2, flush_flags(), 2, &[0xCC, 0xDD]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(&d.platform.storage.data[..4], &[0xAA, 0xBB, 0xCC, 0xDD]);
}

#[test]
fn write_non_sequential_without_flush_rejected() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    d.platform.transport.load_request(Cmd::Write, 0, 4, &[0; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    d.platform
        .transport
        .load_request(Cmd::Write, 128, 4, &[0; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::AddrOutOfBounds);
}

#[test]
fn writes_straddling_pages_are_written_a_page_at_a_time() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    d.platform
        .transport
        .load_request(Cmd::Write, 0, 3, &[1, 2, 3]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    d.platform
        .transport
        .load_request(Cmd::Write, 3, 3, &[4, 5, 6]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    d.platform
        .transport
        .load_request_flags(Cmd::Write, 6, flush_flags(), 3, &[7, 8, 9]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(
        &d.platform.storage.data[..10],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0xFF]
    );
}

#[test]
fn flush_flag_resets_state_for_new_region() {
    let mut p = platform(BootState::Updating);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);

    d.platform
        .transport
        .load_request_flags(Cmd::Write, 0, flush_flags(), 4, &[0xAA; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);

    // After FLUSH, next write can be at any page-aligned address
    d.platform
        .transport
        .load_request_flags(Cmd::Write, 128, flush_flags(), 4, &[0xBB; 4]);
    d.dispatch().unwrap();
    assert_eq!(d.frame.status, Status::Ok);
    assert_eq!(&d.platform.storage.data[128..132], &[0xBB; 4]);
}
