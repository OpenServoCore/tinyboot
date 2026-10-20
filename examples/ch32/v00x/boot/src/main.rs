//! Bootloader example for CH32V00x (V006/V007).
//!
//! Flash-mode features:
//! - `system-flash`: runs from 3328-byte system flash; all 62 KB user flash free for the app.
//! - `user-flash`: occupies first 2 KB of user flash; app gets the remaining 60 KB - 256 B.

#![no_std]
#![no_main]

use panic_halt as _;
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!(compat = "tinyboot-example-v00x");

use tinyboot_ch32::boot::prelude::*;
use tinyboot_ch32::pac::{
    FLASH, RCC,
    rcc::vals::{Hpre, Pllsrc, Sw},
};

fn init_48mhz_hsi_pll() {
    FLASH.actlr().modify(|w| w.set_latency(2));

    RCC.cfgr0().modify(|w| {
        w.set_pllsrc(Pllsrc::HSI);
        w.set_hpre(Hpre::DIV1);
    });

    RCC.ctlr().modify(|w| w.set_pllon(true));
    while !RCC.ctlr().read().pllrdy() {}

    RCC.cfgr0().modify(|w| w.set_sw(Sw::PLL));
    while RCC.cfgr0().read().sws() != Sw::PLL {}
}

#[unsafe(export_name = "main")]
fn main() -> ! {
    init_48mhz_hsi_pll();

    // USART1 transport. Remap options (CH32V00x):
    //   Remap0 (default): TX=PD5, RX=PD6
    //   Remap1:           TX=PD6, RX=PD5
    //   Remap2:           TX=PD0, RX=PD1
    //   Remap3:           TX=PC0, RX=PC1
    //   Remap4:           TX=PD1, RX=PB3
    //   Remap5:           TX=PB3, RX=PD1
    //   Remap6:           TX=PC5, RX=PC6
    //   Remap7:           TX=PB5, RX=PB6
    //   Remap8:           TX=PA0, RX=PA1
    //   Remap9:           TX=PA0, RX=PC4
    //
    // Two-wire UART with an RS-485 DE pin. One wire instead:
    // `wire: HalfDuplex { drive: PushPullWhileSending }` (the wire needs a
    // pull-up); no DE pin: `tx_en: NoTxEn`. See docs/transports.md.
    //
    // rx_pull: Pull::Up for floating lines; Pull::None if externally pulled up.
    let transport = Usart::new(&UsartConfig {
        baud: BaudRate::B3000000,
        pclk: 48_000_000,
        mapping: UsartMapping::Usart1Remap3,
        wire: FullDuplex {
            rx_pull: Pull::None,
            drive: PushPull,
        },
        tx_en: TxEn {
            pin: Pin::PC2,
            tx_level: Level::High,
        },
    });
    // Addressable: silent on a shared bus until a host selects this chip by
    // its unique id; the CLI does that for a single device on its own. One
    // device per bus and nothing to select: `Unaddressed`.
    tinyboot_ch32::boot::run(transport, BootCtl::new(), Addressable::new(chip_uid));
}
