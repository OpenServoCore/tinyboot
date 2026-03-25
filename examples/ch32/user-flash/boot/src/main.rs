//! User-flash bootloader example for CH32V003.
//!
//! The bootloader occupies the first 4KB of user flash, with the application
//! in the remaining 12KB. Boot metadata is stored in option bytes.
//!
//! Because the bootloader runs from user flash, there is no tight size
//! constraint — defmt logging is enabled for easier debugging.
//!
//! Boot requests use a magic word in RAM (preserved across soft resets via a
//! NOLOAD linker section). Both the bootloader and app must link
//! `boot_request.x` to reserve this word.

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use tinyboot_ch32_boot::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Core, Duplex, Platform, Pull, Storage,
    StorageConfig, Usart, UsartConfig, UsartMapping,
};

const APP_BASE: u32 = 0x0800_1000;
const APP_ENTRY: u32 = 0x0000_1000;
const APP_SIZE: usize = 12 * 1024;

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
    let ctl = BootCtl::new(BootCtlConfig {
        app_entry: APP_ENTRY,
    });

    const BOOT_VER: u16 = tinyboot_ch32_boot::pkg_version!();
    let platform = Platform::new(transport, storage, boot_meta, ctl, BOOT_VER);
    Core::new(platform).run();
}
