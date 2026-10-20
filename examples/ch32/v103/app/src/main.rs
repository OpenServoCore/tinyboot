//! Example app for the tinyboot bootloader (CH32V103).
//!
//! - TIM2 interrupt blinks LED on PC13 at 1 Hz.
//! - Main loop listens on USART1 (TX=PA9, RX=PA10) and reboots into the
//!   bootloader when it receives a Reset command.

#![no_std]
#![no_main]

mod transport;

use core::cell::RefCell;

use ch32_hal::gpio::{Level, Output};
use ch32_hal::interrupt::InterruptExt;
use ch32_hal::pac;
use ch32_hal::time::Hertz;
use ch32_hal::timer::low_level::Timer;
use ch32_hal::usart::{self, Uart};
use critical_section::Mutex;

use defmt_rtt as _;

tinyboot_ch32::app::app_version!(compat = "tinyboot-example-v103");

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panic");
    loop {}
}

type Shared<T> = Mutex<RefCell<Option<T>>>;
static LED: Shared<Output<'static>> = Mutex::new(RefCell::new(None));

#[qingke_rt::entry]
fn main() -> ! {
    let p = ch32_hal::init(Default::default());

    // LED blink via TIM2 interrupt (2 Hz toggle = 1 Hz blink)
    critical_section::with(|cs| {
        LED.borrow_ref_mut(cs)
            .replace(Output::new(p.PC13, Level::Low, Default::default()));
    });
    let tim = Timer::new(p.TIM2);
    tim.set_frequency(Hertz::hz(2));
    tim.enable_update_interrupt(true);
    tim.start();
    unsafe { ch32_hal::interrupt::TIM2.enable() };

    // system-flash: pass BOOT0 pin, the level that selects system flash,
    // and the reset delay in cycles (RC ~1ms @ 8MHz = 8000; flip-flop: 0).
    let mut app = tinyboot_ch32::app::new_addressable_app(
        core::cfg_select! {
            feature = "system-flash" => tinyboot_ch32::app::BootCtl::new(
                tinyboot_ch32::app::Pin::PB1,
                tinyboot_ch32::app::Level::High,
                8000,
            ),
            _ => tinyboot_ch32::app::BootCtl::new(),
        },
        wait_bytes,
    );

    // USART1 blocking — must match the bootloader's pin mapping.
    // ch32-hal generic param picks the remap:
    //   0 (default): TX=PA9, RX=PA10
    //   1: TX=PB6, RX=PB7
    // Follow the baud stored for the bootloader (`tinyboot baud`), so one
    // rate reaches the device in either mode.
    let mut uart_config = usart::Config::default();
    uart_config.baudrate = app.baud().map_or(115200, |b| b.hz());
    let uart = Uart::new_blocking::<0>(p.USART1, p.PA10, p.PA9, uart_config).unwrap();
    let (tx, rx) = uart.split();
    let mut rx = transport::Rx(rx);

    // RS-485 DE/RE on PC2, matching the bootloader. tx_level=Low means idle-RX
    // drives the pin High (inverse), which tri-states the transceiver so the
    // programmer UART TX can reach MCU_RX without contention.
    let tx_level = Level::Low;
    let tx_en_pin = Output::new(p.PC2, transport::invert(tx_level), Default::default());
    let mut tx = transport::Tx {
        uart: tx,
        tx_en: Some(transport::TxEn {
            pin: tx_en_pin,
            tx_level,
        }),
    };

    app.confirm();

    defmt::info!("Boot confirmed, app ready.");

    loop {
        app.poll(&mut rx, &mut tx);
    }
}

/// Stagger `Select` replies: about `n` byte-times at USART1's baud (BRR
/// is clock cycles per bit).
fn wait_bytes(n: u8) {
    let bit = pac::USART1.brr().read().0;
    for _ in 0..n as u32 * 10 * bit {
        // SAFETY: empty asm, only keeps the loop.
        unsafe { core::arch::asm!("") };
    }
}

#[qingke_rt::interrupt]
fn TIM2() {
    pac::TIM2.intfr().modify(|w| w.set_uif(false));
    critical_section::with(|cs| {
        if let Some(ref mut led) = *LED.borrow_ref_mut(cs) {
            led.toggle();
            if led.is_set_high() {
                defmt::info!("LED on");
            } else {
                defmt::info!("LED off");
            }
        }
    });
}
