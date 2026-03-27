//! Bootloader example for CH32V003.
//!
//! Two flash modes available via feature flags:
//!
//! **system-flash**: Runs from the 1920-byte system flash region, leaving all
//! 16KB of user flash for the application. No defmt (size constrained).
//!
//! **user-flash**: Occupies first 8KB of user flash, with the application in
//! the remaining 8KB. defmt enabled for debugging.

#![no_std]
#![no_main]

#[cfg(feature = "system-flash")]
use panic_halt as _;

#[cfg(feature = "user-flash")]
use defmt_rtt as _;

tinyboot_ch32_boot::boot_version!();

#[cfg(feature = "user-flash")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panic: {}", defmt::Display2Format(info));
    loop {}
}

use tinyboot_ch32_boot::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Duplex, Platform, Pull, Storage,
    StorageConfig, Usart, UsartConfig, UsartMapping,
};

#[cfg(feature = "system-flash")]
const APP_BASE: u32 = 0x0800_0000;
#[cfg(feature = "system-flash")]
const APP_SIZE: usize = 16 * 1024;

#[cfg(feature = "user-flash")]
const APP_BASE: u32 = 0x0800_2000;
#[cfg(feature = "user-flash")]
const APP_ENTRY: u32 = 0x0000_2000;
#[cfg(feature = "user-flash")]
const APP_SIZE: usize = 8 * 1024;

#[unsafe(export_name = "main")]
fn main() -> ! {
    // USART1 transport for firmware updates.
    //
    // Remap options (CH32V003):
    //   Remap0: TX=PD5, RX=PD6 (default)
    //   Remap1: TX=PD0, RX=PD1
    //   Remap2: TX=PD6, RX=PD5
    //   Remap3: TX=PC0, RX=PC1
    //
    // rx_pull: Pull::Up for floating RX lines, Pull::None if external pullup present.
    //
    // For RS-485 half-duplex with a transceiver DE pin:
    //   duplex: Duplex::Half,
    //   tx_en: Some(TxEnConfig { pin: Pin::PC2, active_high: true }),
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        rx_pull: Pull::None,
        tx_en: None,
    });

    let storage = Storage::new(StorageConfig {
        app_base: APP_BASE,
        app_size: APP_SIZE,
    });
    let boot_meta = BootMetaStore::default();

    #[cfg(feature = "system-flash")]
    let ctl = BootCtl::new(BootCtlConfig {});

    #[cfg(feature = "user-flash")]
    let ctl = BootCtl::new(BootCtlConfig {
        app_entry: APP_ENTRY,
    });

    let platform = Platform::new(transport, storage, boot_meta, ctl);
    tinyboot_ch32_boot::run(platform);
}
