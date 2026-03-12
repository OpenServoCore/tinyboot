#![no_std]
#![no_main]

use panic_halt as _;

use ch32_metapac::gpio::vals::{Cnf, Mode};
use embedded_io::{Read, Write};
use qingke_rt::entry;
use tiny_boot_ch32::hal::transport::usart::{BaudRate, Duplex, Usart, UsartConfig};

#[entry]
fn main() -> ! {
    // Enable clocks: GPIOD + AFIO + USART1
    let rcc = ch32_metapac::RCC;
    rcc.apb2pcenr().modify(|w| {
        w.set_iopden(true);
        w.set_afioen(true);
        w.set_usart1en(true);
    });

    // PD5 = TX: alternate function push-pull, 50MHz
    let gpiod = ch32_metapac::GPIOD;
    gpiod.cfglr().modify(|w| {
        w.set_mode(5, Mode::OUTPUT_50MHZ);
        w.set_cnf(5, Cnf::PULL_IN__AF_PUSH_PULL_OUT);
    });
    // PD6 = RX: input floating
    gpiod.cfglr().modify(|w| {
        w.set_mode(6, Mode::INPUT);
        w.set_cnf(6, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
    });

    let config = UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 48_000_000, // testing: maybe PLL is active?
    };
    let mut usart = Usart::new(ch32_metapac::USART1, &config);

    // Send 'U' (0x55) continuously — produces a square wave on TX
    // making it easy to verify baud rate with a scope or detect mismatches
    loop {
        let _ = usart.write(b"U");
        let _ = usart.flush();
    }
}
