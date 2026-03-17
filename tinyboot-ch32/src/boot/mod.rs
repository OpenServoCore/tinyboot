pub mod platform;
mod rt;

pub use platform::{
    BaudRate, BootCtl, BootMetaStore, Duplex, MetaConfig, Pin, Pull, Storage, StorageConfig,
    TxEnConfig, Usart, UsartConfig, UsartMapping,
};
