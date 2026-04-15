//! Bootloader example for CH32V103.
//!
//! Two flash modes available via feature flags:
//!
//! **system-flash**: Runs from the 2048-byte system flash region, leaving all
//! 64KB of user flash for the application. Requires external BOOT0 control
//! circuit (RC or flip-flop) on the configured GPIO pin.
//!
//! **user-flash**: Occupies first 8KB of user flash, with the application in
//! the remaining 56KB.

#![no_std]
#![no_main]

use panic_halt as _;

tinyboot_ch32_boot::boot_version!();

use tinyboot_ch32_boot::prelude::*;

#[unsafe(export_name = "main")]
fn main() -> ! {
    // USART1 transport for firmware updates.
    //
    // Remap options (CH32V103):
    //   Remap0: TX=PA9, RX=PA10 (default)
    //   Remap1: TX=PB6, RX=PB7
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        rx_pull: Pull::None,
        tx_en: None,
    });

    // V103 system-flash: configure GPIO pin driving the external BOOT0
    // control circuit (RC or flip-flop). Adjust pin to your hardware.
    #[cfg(feature = "system-flash")]
    let config = BootCtlConfig {
        pin: Pin::PB1,     // adjust to your BOOT0 control pin
        active_high: true, // RC circuit: HIGH = system flash
    };

    // V103 user-flash: no GPIO boot control needed, uses RAM magic word.
    #[cfg(not(feature = "system-flash"))]
    let config = BootCtlConfig;

    tinyboot_ch32_boot::run(transport, config);
}
