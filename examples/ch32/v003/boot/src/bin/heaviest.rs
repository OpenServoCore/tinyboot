//! Size gate, built by CI beside the example: every option that costs
//! flash at once (one-wire push-pull, DE pin, compatibility key,
//! addressable, a 256-byte payload). Its system-flash build must still
//! link into the 1920-byte system flash.

#![no_std]
#![no_main]

use panic_halt as _;
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!(compat = "tinyboot-example-v003");

use tinyboot_ch32::boot::prelude::*;

#[unsafe(export_name = "main")]
fn main() -> ! {
    let transport = Usart::new(&UsartConfig {
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        wire: HalfDuplex {
            drive: PushPullWhileSending,
        },
        tx_en: TxEn {
            pin: Pin::PC2,
            tx_level: Level::Low,
        },
    });
    tinyboot_ch32::boot::run_with_payload::<256>(
        transport,
        BootCtl::new(),
        Addressable::new(chip_uid),
    );
}
