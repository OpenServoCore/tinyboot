pub mod platform;
#[cfg(all(target_arch = "riscv32", feature = "rt"))]
mod rt;

pub use platform::{
    BaudRate, BootCtl, BootMetaStore, Duplex, MetaConfig, Pin, Pull, Storage, StorageConfig,
    TxEnConfig, Usart, UsartConfig, UsartMapping,
};
