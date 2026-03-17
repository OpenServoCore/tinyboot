pub mod platform;
#[cfg(target_arch = "riscv32")]
mod rt;

pub use platform::{
    BaudRate, BootCtl, BootMetaStore, Duplex, MetaConfig, Pin, Pull, Storage, StorageConfig,
    TxEnConfig, Usart, UsartConfig, UsartMapping,
};
