#![no_std]
#![warn(missing_docs)]

//! Platform-agnostic bootloader core.
//!
//! Implements the boot state machine, protocol dispatcher, and app validation.
//! Platform-specific behaviour is injected via the traits in [`traits::boot`].

/// App-side tinyboot client (poll, confirm, command handling).
pub mod app;
/// Boot state machine and entry point.
pub mod core;
/// Protocol frame dispatcher.
pub mod protocol;
/// Fixed-size ring buffer for buffered flash writes.
pub mod ringbuf;
/// Platform abstraction traits.
pub mod traits;

pub use crate::core::Core;

// Re-export so version macros can use `$crate::pkg_version!()`.
#[doc(hidden)]
pub use tinyboot_protocol::pkg_version;

/// Read the version from the `__tinyboot_version` linker symbol.
///
/// This symbol is defined by `tb-boot.x` / `tb-app.x` and points to the
/// `.tinyboot_version` section populated by [`boot_version!`] or [`app_version!`].
#[inline(always)]
pub fn tinyboot_version() -> u16 {
    unsafe extern "C" {
        static __tinyboot_version: u16;
    }
    unsafe { ::core::ptr::read_volatile(&raw const __tinyboot_version) }
}

/// Define the `.tinyboot_version` static using the calling crate's version.
/// Place this at module scope in your bootloader binary.
#[macro_export]
macro_rules! boot_version {
    () => {
        #[unsafe(link_section = ".tinyboot_version")]
        #[used]
        static _BOOT_VERSION: u16 = $crate::pkg_version!();
    };
}

/// Define the `.tinyboot_version` static using the calling crate's version.
/// Place this at module scope in your application binary.
#[macro_export]
macro_rules! app_version {
    () => {
        #[unsafe(link_section = ".tinyboot_version")]
        #[used]
        static _APP_VERSION: u16 = $crate::pkg_version!();
    };
}
