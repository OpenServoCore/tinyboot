#![no_std]

pub mod platform;

#[cfg(all(target_arch = "riscv32", feature = "rt"))]
mod rt;

pub use platform::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Duplex, MetaConfig, Storage, StorageConfig,
    TxEnConfig, Usart, UsartConfig,
};

// Re-export HAL types for convenience
pub use tinyboot_ch32_hal::gpio::Pull;
pub use tinyboot_ch32_hal::{Pin, UsartMapping};
