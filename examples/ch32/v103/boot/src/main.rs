//! Bootloader example for CH32V103.
//!
//! Flash-mode features:
//! - `system-flash`: runs from 2048-byte system flash; all 64 KB user flash free for the app.
//!   Requires an external BOOT0 control circuit (RC or flip-flop) on the configured GPIO.
//! - `user-flash`: occupies first 8 KB of user flash; app gets the remaining 56 KB.

#![no_std]
#![no_main]

use panic_halt as _;
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!(compat = "tinyboot-example-v103");

use tinyboot_ch32::boot::prelude::*;

#[unsafe(export_name = "main")]
fn main() -> ! {
    // USART1 transport. Remap options (CH32V103):
    //   Remap0 (default): TX=PA9, RX=PA10
    //   Remap1:           TX=PB6, RX=PB7
    //
    // Two-wire UART with an RS-485 DE pin. One wire instead:
    // `wire: HalfDuplex { drive: PushPullWhileSending }` (the wire needs a
    // pull-up); no DE pin: `tx_en: NoTxEn`. See docs/transports.md.
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

    // system-flash: GPIO drives the external BOOT0 circuit. The Level arg is
    // the pin state that selects the system-flash bootloader; delay is
    // circuit settle time in CPU cycles (RC ~1ms @ 8MHz = 8000; flip-flop: 0).
    // user-flash: no GPIO needed — run-mode lives in a RAM magic word.
    let ctl = core::cfg_select! {
        feature = "system-flash" => BootCtl::new(Pin::PB1, Level::High, 8000),
        _ => BootCtl::new(),
    };

    // Addressable: silent on a shared bus until a host selects this chip by
    // its unique id; the CLI does that for a single device on its own. One
    // device per bus and nothing to select: `Unaddressed`.
    tinyboot_ch32::boot::run(transport, ctl, Addressable::new(chip_uid));
}
