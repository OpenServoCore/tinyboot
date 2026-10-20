//! CH32 bootloader entry point.

use crate::platform::{BootMetaStore, Storage};

pub use crate::platform::{
    BaudRate, BootCtl, FullDrive, FullDuplex, HalfDrive, HalfDuplex, NoTxEn, OpenDrain, PushPull,
    PushPullWhileSending, TxEn, TxEnable, Usart, UsartConfig, Wire,
};

pub use crate::hal::gpio::{Level, Pull};
pub use crate::hal::{Pin, UsartMapping};
pub use tinyboot_core::Platform;
use tinyboot_core::frame::{DEFAULT_PAYLOAD, InfoData};
pub use tinyboot_core::{boot_version, pkg_version};

/// Common imports for bootloader binaries.
pub mod prelude {
    pub use super::{
        BaudRate, BootCtl, FullDuplex, HalfDuplex, Level, NoTxEn, OpenDrain, Pin, Pull, PushPull,
        PushPullWhileSending, TxEn, Usart, UsartConfig, UsartMapping,
    };
}

pub const PAGE_SIZE: usize = crate::hal::flash::PAGE_SIZE;

/// Run the bootloader with the default 52-byte frame payload. Does not
/// return.
#[inline(always)]
pub fn run(transport: impl tinyboot_core::traits::Transport, ctl: BootCtl) -> ! {
    run_with_payload::<DEFAULT_PAYLOAD>(transport, ctl)
}

/// Run the bootloader with a `P`-byte frame payload, at least the 18-byte
/// Info reply and at most a flash page. A larger payload takes fewer
/// Writes for an image and costs `P` bytes of RAM. Does not return.
#[inline(always)]
pub fn run_with_payload<const P: usize>(
    transport: impl tinyboot_core::traits::Transport,
    ctl: BootCtl,
) -> ! {
    const {
        assert!(P >= InfoData::SIZE as usize && P <= PAGE_SIZE);
    }
    let platform = Platform::new(transport, Storage::default(), BootMetaStore::default(), ctl);
    tinyboot_core::Core::<_, _, _, _, { 2 * PAGE_SIZE }, P>::new(platform).run()
}
