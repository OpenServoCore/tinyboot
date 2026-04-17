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
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!();

use tinyboot_ch32::boot::prelude::*;

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
    // control circuit (RC or flip-flop). Adjust pin + reset delay to your
    // hardware (RC: ~1ms settle at 8MHz = 8000 cycles; flip-flop: 0).
    #[cfg(feature = "system-flash")]
    let ctl = BootCtl::new(Pin::PB1, true, 8000);

    // V103 user-flash: no GPIO boot control needed, uses RAM magic word.
    #[cfg(not(feature = "system-flash"))]
    let ctl = BootCtl::new();

    tinyboot_ch32::boot::run(transport, ctl);
}
