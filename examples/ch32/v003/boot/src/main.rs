//! Bootloader example for CH32V003.
//!
//! Two flash modes available via feature flags:
//!
//! **system-flash**: Runs from the 1920-byte system flash region, leaving all
//! 16KB of user flash for the application.
//!
//! **user-flash**: Occupies first 2KB of user flash, with the application in
//! the remaining 14KB.

#![no_std]
#![no_main]

use panic_halt as _;

tinyboot_ch32_boot::boot_version!();

use tinyboot_ch32_boot::prelude::*;

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
    tinyboot_ch32_boot::run(transport);
}
