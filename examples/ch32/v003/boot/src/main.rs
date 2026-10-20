//! Bootloader example for CH32V003.
//!
//! Flash-mode features:
//! - `system-flash`: runs from 1920-byte system flash; all 16 KB user flash free for the app.
//! - `user-flash`: occupies first 2 KB of user flash; app gets the remaining 14 KB.

#![no_std]
#![no_main]

use panic_halt as _;
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!();

use tinyboot_ch32::boot::prelude::*;

#[unsafe(export_name = "main")]
fn main() -> ! {
    // USART1 transport. Remap options (CH32V003):
    //   Remap0 (default): TX=PD5, RX=PD6
    //   Remap1:           TX=PD0, RX=PD1
    //   Remap2:           TX=PD6, RX=PD5
    //   Remap3:           TX=PC0, RX=PC1
    //
    // Two-wire UART with an RS-485 DE pin. One wire instead:
    // `wire: HalfDuplex { drive: PushPullWhileSending }` (the wire needs a
    // pull-up); no DE pin: `tx_en: NoTxEn`. See docs/transports.md.
    //
    // rx_pull: Pull::Up for floating lines; Pull::None if externally pulled up.
    let transport = Usart::new(&UsartConfig {
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        wire: FullDuplex {
            rx_pull: Pull::None,
            drive: PushPull,
        },
        tx_en: TxEn {
            pin: Pin::PC2,
            tx_level: Level::Low,
        },
    });
    tinyboot_ch32::boot::run(transport, BootCtl::new());
}
