#![no_std]
#![warn(missing_docs)]

//! Platform-agnostic bootloader core.
//!
//! Boot state machine, protocol dispatcher, app validation. Platform-specific
//! behaviour is injected via the traits in [`traits`].

/// App-side tinyboot client.
pub mod app;
/// Boot state machine and entry point.
pub mod core;
/// Boot-time platform container.
pub mod platform;
/// Protocol frame dispatcher.
pub mod protocol;
/// Fixed-size ring buffer for buffered flash writes.
pub mod ringbuf;
/// Platform abstraction traits.
pub mod traits;

pub use crate::core::Core;
pub use crate::platform::Platform;

#[doc(hidden)]
pub use tinyboot_protocol::pkg_version;

/// Read the version packed into the `.tb_version` section.
#[inline(always)]
pub fn tinyboot_version() -> u16 {
    unsafe extern "C" {
        static __tb_version: u16;
    }
    unsafe { ::core::ptr::read_volatile(&raw const __tb_version) }
}

/// Place the calling crate's version into `.tb_version`. Use at module scope
/// in a bootloader binary.
#[macro_export]
macro_rules! boot_version {
    () => {
        #[unsafe(link_section = ".tb_version")]
        #[used]
        static _BOOT_VERSION: u16 = $crate::pkg_version!();
    };
}

/// Place the calling crate's version into `.tb_version`. Use at module scope
/// in an app binary.
#[macro_export]
macro_rules! app_version {
    () => {
        #[unsafe(link_section = ".tb_version")]
        #[used]
        static _APP_VERSION: u16 = $crate::pkg_version!();
    };
}
