#![no_std]
#![warn(missing_docs)]

//! Platform-agnostic bootloader core.
//!
//! Boot state machine, protocol dispatcher, app validation. Platform-specific
//! behaviour is injected via the traits in [`traits`].

/// Bus addressing: one device per bus, or many selected by unique id.
pub mod addressing;
/// App-side tinyboot client.
pub mod app;
/// Boot state machine and entry point.
pub mod core;
/// Boot-time platform container.
pub mod platform;
/// Protocol frame dispatcher.
pub mod protocol;
/// Platform abstraction traits.
pub mod traits;

pub use crate::core::Core;
pub use crate::platform::Platform;
pub use tinyboot_protocol::BaudRate;

pub use tinyboot_protocol::frame;
#[doc(hidden)]
pub use tinyboot_protocol::pkg_version;
pub use tinyboot_protocol::{COMPAT_UNSET, compat_key};

/// Last 6 bytes of a boot or app image, emitted by [`boot_version!`] or
/// [`app_version!`] into `.tb_version`. `tb-app.x` starts an app's tail
/// word-aligned, so the bootloader reads its key with one word load.
#[repr(C, packed(2))]
pub struct ImageTail {
    /// Compatibility key; [`COMPAT_UNSET`](tinyboot_protocol::COMPAT_UNSET)
    /// when none is set.
    pub compat: u32,
    /// Packed 5.5.6 version.
    pub version: u16,
}

unsafe extern "C" {
    static __tb_tail: ImageTail;
}

/// This image's version, from its tail.
#[inline(always)]
pub fn tinyboot_version() -> u16 {
    unsafe { __tb_tail.version }
}

/// This image's compatibility key, from its tail. A plain load, so with
/// LTO it folds to a constant and an unset key compiles checks out.
#[inline(always)]
pub fn tinyboot_compat() -> u32 {
    unsafe { __tb_tail.compat }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __image_tail {
    ($compat:expr) => {
        #[unsafe(export_name = "__tb_tail")]
        #[unsafe(link_section = ".tb_version")]
        #[used]
        static _TB_TAIL: $crate::ImageTail = $crate::ImageTail {
            compat: $compat,
            version: $crate::pkg_version!(),
        };
    };
}

/// Place the calling crate's version and a compatibility key into the
/// bootloader image's tail. Use once at module scope in a bootloader
/// binary.
///
/// - `boot_version!()`: no key; the bootloader accepts any image.
/// - `boot_version!(compat = "name")`: the key hashed from a name
///   ([`compat_key`](tinyboot_protocol::compat_key)).
/// - `boot_version!(compat_key = 0x1234_5678)`: a raw key.
///
/// With a key set, `Verify` refuses an image whose key differs with
/// `Status::Incompatible`.
#[macro_export]
macro_rules! boot_version {
    () => {
        $crate::__image_tail!($crate::COMPAT_UNSET);
    };
    (compat = $name:expr) => {
        $crate::__image_tail!($crate::compat_key($name));
    };
    (compat_key = $key:expr) => {
        $crate::__image_tail!($key);
    };
}

/// Place the calling crate's version and a compatibility key into the app
/// image's tail. Use once at module scope in an app binary. Arguments as
/// [`boot_version!`]; the key must equal the bootloader's for it to
/// accept the image.
#[macro_export]
macro_rules! app_version {
    () => {
        $crate::__image_tail!($crate::COMPAT_UNSET);
    };
    (compat = $name:expr) => {
        $crate::__image_tail!($crate::compat_key($name));
    };
    (compat_key = $key:expr) => {
        $crate::__image_tail!($key);
    };
}
