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
pub use tinyboot::Core;
pub use tinyboot::traits::boot::Platform;
pub use tinyboot_ch32_hal::gpio::Pull;
pub use tinyboot_ch32_hal::{Pin, UsartMapping};
pub use tinyboot_protocol::pkg_version;

#[unsafe(link_section = ".tinyboot_version")]
#[used]
static BOOT_VERSION: u16 = tinyboot_protocol::pkg_version!();
