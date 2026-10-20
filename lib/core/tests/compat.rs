//! Verify against a bootloader built with a compatibility key.

mod common;

use common::*;
use tinyboot_core::traits::BootState;
use tinyboot_protocol::{COMPAT_UNSET, Status, compat_key};

tinyboot_core::boot_version!(compat = "board-a");

#[test]
fn verify_accepts_the_same_key() {
    let mut p = platform(BootState::Updating);
    load_tagged_image(&mut p.storage, 14, compat_key("board-a"));
    assert_eq!(verify(&mut p, 14), Status::Ok);
    assert_eq!(p.boot_meta.state, BootState::Validating);
}

#[test]
fn verify_refuses_another_compat_key() {
    let mut p = platform(BootState::Updating);
    load_tagged_image(&mut p.storage, 14, compat_key("board-b"));
    assert_eq!(verify(&mut p, 14), Status::Incompatible);
    assert_eq!(p.boot_meta.state, BootState::Updating);
    assert_eq!(p.boot_meta.checksum, 0xFFFF);
}

#[test]
fn verify_refuses_an_image_without_a_key() {
    let mut p = platform(BootState::Updating);
    load_tagged_image(&mut p.storage, 14, COMPAT_UNSET);
    assert_eq!(verify(&mut p, 14), Status::Incompatible);
}

#[test]
fn verify_refuses_a_size_that_cannot_end_in_a_tail() {
    for size in [1, 2, 4, 8, 13] {
        let mut p = platform(BootState::Updating);
        assert_eq!(verify(&mut p, size), Status::Incompatible, "size {size}");
        assert_eq!(p.boot_meta.state, BootState::Updating);
    }
}

#[test]
fn info_reports_the_key() {
    let mut p = platform(BootState::Idle);
    let mut d = tinyboot_core::protocol::Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    d.platform
        .transport
        .load_request(tinyboot_protocol::Cmd::Info, 0, 0, &[]);
    d.dispatch().unwrap();
    assert_eq!(
        { unsafe { d.frame.data.info }.compat },
        compat_key("board-a")
    );
}
