#![no_std]
#![no_main]

use panic_halt as _;

#[cfg(feature = "defmt")]
use defmt_rtt as _;

use tinyboot::{Core, traits::Platform};
use tinyboot_ch32::boot::{
    BaudRate, BootCtl, BootMetaStore, Duplex, MetaConfig, Pin, Pull, Storage, StorageConfig,
    TxEnConfig, Usart, UsartConfig, UsartMapping,
};

#[unsafe(export_name = "main")]
fn main() -> ! {
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap3,
        rx_pull: Pull::Up,
        tx_en: Some(TxEnConfig {
            pin: Pin::PC2,
            active_high: true,
        }),
    });

    let storage = Storage::new(StorageConfig {
        app_base: 0x0800_0000,
        app_size: 16 * 1024,
    });
    let boot_meta = BootMetaStore::new(MetaConfig {
        meta_base: 0x1FFF_FCC0,
    });
    let ctl = BootCtl;

    let platform = Platform::new(transport, storage, boot_meta, ctl);
    Core::new(platform).run();
}
