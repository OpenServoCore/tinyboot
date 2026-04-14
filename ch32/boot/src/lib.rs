#![no_std]
#![warn(missing_docs)]

//! CH32 bootloader platform implementation.
//!
//! Provides storage, transport, boot control, and metadata backed by the
//! CH32 flash controller.

/// Platform components (storage, transport, boot control, metadata).
pub mod platform;

#[cfg(all(target_arch = "riscv32", feature = "rt"))]
mod rt;

pub use platform::{
    BaudRate, BootCtl, BootMetaStore, Duplex, Storage, TxEnConfig, Usart, UsartConfig,
};

// Re-exports so boot examples only need this one crate.
pub use tinyboot::traits::boot::Platform;
pub use tinyboot::{boot_version, pkg_version};
pub use tinyboot_ch32_hal::gpio::Pull;
pub use tinyboot_ch32_hal::{Pin, UsartMapping};

/// Common imports for bootloader binaries.
pub mod prelude {
    pub use crate::{BaudRate, Duplex, Pin, Pull, TxEnConfig, Usart, UsartConfig, UsartMapping};
}

/// Protocol write buffer size (2 × page size).
const PROTOCOL_BUF_SIZE: usize = 2 * tinyboot_ch32_hal::flash::PAGE_SIZE;

/// Run the bootloader with the given transport.
///
/// Sets up storage, boot metadata, and boot control from linker symbols,
/// then enters the boot state machine. Does not return.
#[inline(always)]
pub fn run(transport: impl tinyboot::traits::boot::Transport) -> ! {
    let platform = Platform::new(
        transport,
        Storage::default(),
        BootMetaStore::default(),
        BootCtl::default(),
    );
    tinyboot::Core::<_, _, _, _, PROTOCOL_BUF_SIZE>::new(platform).run()
}
