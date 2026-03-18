//! User-flash bootloader example for CH32V003.
//!
//! The bootloader occupies the first 8KB of user flash, with the application
//! in the remaining 8KB (minus 64 bytes for boot metadata at the end).
//!
//! Because the bootloader runs from user flash, there is no tight size
//! constraint — defmt logging is enabled for easier debugging.
//!
//! Without the `system-flash` feature, boot requests use a magic word in RAM
//! (preserved across soft resets via a NOLOAD linker section). Both the
//! bootloader and app must link `boot_request.x` to reserve this word.

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use tinyboot::{Core, traits::Platform};
use tinyboot_ch32_boot::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Duplex, MetaConfig, Pull, Storage,
    StorageConfig, Usart, UsartConfig, UsartMapping,
};

// --- Flash layout (must match memory.x) ---

/// Application entry point (execution alias).
const APP_ENTRY: u32 = 0x0000_1000;

/// Application FPEC programming address (0x0800_0000 base).
const APP_BASE: u32 = 0x0800_1000;

/// Full 12KB available for the application.
const APP_SIZE: usize = 12 * 1024;

/// Boot metadata at the last 64 bytes of the bootloader's 4KB region.
/// Must match the META origin in memory.x.
const META_BASE: u32 = 0x0800_0FC0;

#[unsafe(export_name = "main")]
fn main() -> ! {
    // Configure USART transport for firmware updates.
    // Adjust mapping, pins, and baud rate to match your hardware.
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,                     // HSI default clock, no PLL
        mapping: UsartMapping::Usart1Remap3, // TX=PD6, RX=PD5
        rx_pull: Pull::None,
        // For RS-485 half-duplex, enable tx_en to drive a transceiver DE pin:
        // tx_en: Some(TxEnConfig { pin: Pin::PC2, active_high: true }),
        tx_en: None,
    });

    let storage = Storage::new(StorageConfig {
        app_base: APP_BASE,
        app_size: APP_SIZE,
    });
    let boot_meta = BootMetaStore::new(MetaConfig {
        meta_base: META_BASE,
    });
    let ctl = BootCtl::new(BootCtlConfig {
        app_entry: APP_ENTRY,
    });

    let platform = Platform::new(transport, storage, boot_meta, ctl);
    Core::new(platform).run();
}
