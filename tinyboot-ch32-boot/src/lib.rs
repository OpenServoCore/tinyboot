#![no_std]
#![warn(missing_docs)]

//! CH32 bootloader platform implementation.
//!
//! Provides storage, transport, boot control, and metadata backed by the
//! CH32 flash controller and option bytes.

/// Platform components (storage, transport, boot control, metadata).
pub mod platform;

#[cfg(all(target_arch = "riscv32", feature = "rt"))]
mod rt;

pub use platform::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Duplex, Storage, StorageConfig, TxEnConfig,
    Usart, UsartConfig,
};

// Re-exports so boot examples only need this one crate.
pub use tinyboot::traits::boot::Platform;
pub use tinyboot::{boot_version, pkg_version};
pub use tinyboot_ch32_hal::gpio::Pull;
pub use tinyboot_ch32_hal::{Pin, UsartMapping};

/// Protocol write buffer size (2 × page size).
const PROTOCOL_BUF_SIZE: usize = 2 * tinyboot_ch32_hal::flash::PAGE_SIZE;

/// Run the bootloader. Hides the protocol buffer size const generic.
#[inline(always)]
pub fn run(platform: Platform<Usart, Storage, BootMetaStore, BootCtl>) -> ! {
    tinyboot::Core::<_, _, _, _, PROTOCOL_BUF_SIZE>::new(platform).run()
}
