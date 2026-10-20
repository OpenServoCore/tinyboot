//! An image tail with its own version instead of the crate's.

use tinyboot_protocol::{compat_key, pack_version};

tinyboot_core::app_version!(compat = "board-a", version = pack_version(3, 1, 4));

#[test]
fn the_tail_carries_the_given_version_and_key() {
    assert_eq!(tinyboot_core::tinyboot_version(), pack_version(3, 1, 4));
    assert_eq!(tinyboot_core::tinyboot_compat(), compat_key("board-a"));
}
