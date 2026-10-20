//! CH32 bootloader entry point.

use crate::platform::{BootMetaStore, Storage};

pub use crate::platform::{
    BaudRate, BootCtl, FullDrive, FullDuplex, HalfDrive, HalfDuplex, NoTxEn, OpenDrain, PushPull,
    PushPullWhileSending, TxEn, TxEnable, Usart, UsartConfig, Wire,
};

pub use crate::hal::esig::uid as chip_uid;
pub use crate::hal::gpio::{Level, Pull};
pub use crate::hal::{Pin, UsartMapping};
pub use tinyboot_core::Platform;
pub use tinyboot_core::addressing::{Addressable, Addressing, Unaddressed};
use tinyboot_core::frame::{DEFAULT_PAYLOAD, InfoData};
pub use tinyboot_core::{boot_version, compat_key, pack_version, pkg_version};

/// Common imports for bootloader binaries.
pub mod prelude {
    pub use super::{
        Addressable, BaudRate, BootCtl, FullDuplex, HalfDuplex, Level, NoTxEn, OpenDrain, Pin,
        Pull, PushPull, PushPullWhileSending, TxEn, Unaddressed, Usart, UsartConfig, UsartMapping,
        chip_uid,
    };
}

pub const PAGE_SIZE: usize = crate::hal::flash::PAGE_SIZE;

/// Run the bootloader with the default 52-byte frame payload. Does not
/// return.
///
/// `addressing` is [`Unaddressed`] for one device per bus, or
/// `Addressable::new(chip_uid)` for many devices on one bus, each
/// silent until a host selects it by its unique id.
#[inline(always)]
pub fn run(
    transport: impl tinyboot_core::traits::Transport,
    ctl: BootCtl,
    addressing: impl Addressing,
) -> ! {
    run_with_payload::<DEFAULT_PAYLOAD>(transport, ctl, addressing)
}

/// Run the bootloader with a `P`-byte frame payload, at least the 18-byte
/// Info reply; a Write may span flash pages. A larger payload takes fewer
/// Writes for an image and costs `P` bytes of RAM. Does not return.
#[inline(always)]
pub fn run_with_payload<const P: usize>(
    transport: impl tinyboot_core::traits::Transport,
    ctl: BootCtl,
    addressing: impl Addressing,
) -> ! {
    const {
        assert!(P >= InfoData::SIZE as usize);
    }
    let platform = Platform::new(transport, Storage::default(), BootMetaStore::default(), ctl);
    tinyboot_core::Core::<_, _, _, _, PAGE_SIZE, P, _>::with_addressing(platform, addressing).run()
}
