//! CH32 bootloader entry point.

use crate::platform::{BootMetaStore, Storage};

pub use crate::platform::{BaudRate, BootCtl, Duplex, TxEnConfig, Usart, UsartConfig};

pub use crate::hal::gpio::{Level, Pull};
pub use crate::hal::{Pin, UsartMapping};
pub use tinyboot_core::Platform;
pub use tinyboot_core::{boot_version, pkg_version};

/// Common imports for bootloader binaries.
pub mod prelude {
    pub use super::{
        BaudRate, BootCtl, Duplex, Level, Pin, Pull, TxEnConfig, Usart, UsartConfig, UsartMapping,
    };
}

pub const PAGE_SIZE: usize = crate::hal::flash::PAGE_SIZE;

/// Run the bootloader. Does not return.
#[inline(always)]
pub fn run(transport: impl tinyboot_core::traits::Transport, ctl: BootCtl) -> ! {
    let platform = Platform::new(transport, Storage::default(), BootMetaStore::default(), ctl);
    tinyboot_core::Core::<_, _, _, _, { 2 * PAGE_SIZE }>::new(platform).run()
}
