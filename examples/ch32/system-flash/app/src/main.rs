//! Example application for the system-flash bootloader.
//!
//! - Timer interrupt blinks LED on PD4 every second
//! - Main loop listens on USART1 (TX=PD5, RX=PD6) for tinyboot commands,
//!   reboots into bootloader on receipt of Reset command
//!
//! No async runtime — just a timer interrupt for blink and a blocking main loop.

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
tinyboot_ch32_app::app_version!();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panic: {}", defmt::Display2Format(info));
    loop {}
}

// --- Flash layout (must match bootloader) ---
const BOOT_BASE: u32 = 0x1FFF_F000;
const BOOT_SIZE: u32 = 1920;
const APP_SIZE: u32 = 16 * 1024;
const ERASE_SIZE: u16 = 64;

type Shared<T> = Mutex<RefCell<Option<T>>>;
static LED: Shared<Output<'static>> = Mutex::new(RefCell::new(None));

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

#[qingke_rt::entry]
fn main() -> ! {
    let p = ch32_hal::init(Default::default());

    // LED blink via TIM2 interrupt (2 Hz toggle = 1 Hz blink)
    critical_section::with(|cs| {
        LED.borrow_ref_mut(cs)
            .replace(Output::new(p.PD4, Level::Low, Default::default()));
    });
    let tim = Timer::new(p.TIM2);
    tim.set_frequency(Hertz::hz(2));
    tim.enable_update_interrupt(true);
    tim.start();
    unsafe { ch32_hal::interrupt::TIM2.enable() };

    // USART1 blocking — must match the bootloader's pin mapping.
    //
    // Remap options (CH32V003, ch32-hal generic param):
    //   0 (Remap0): TX=PD5, RX=PD6 (default)
    //   1 (Remap1): TX=PD0, RX=PD1
    //   2 (Remap2): TX=PD6, RX=PD5
    //   3 (Remap3): TX=PC0, RX=PC1
    let mut uart_config = usart::Config::default();
    uart_config.baudrate = 115200;
    let uart = Uart::new_blocking::<0>(p.USART1, p.PD6, p.PD5, uart_config).unwrap();
    let (tx, rx) = uart.split();
    let mut rx = transport::Rx(rx);
    let mut tx = transport::Tx(tx);

    // Tinyboot app client
    let mut app = tinyboot_ch32_app::new_app(BOOT_BASE, BOOT_SIZE, APP_SIZE, ERASE_SIZE);
    app.confirm();
    defmt::info!("Boot confirmed, app ready.");

    loop {
        app.poll(&mut rx, &mut tx);
    }
}
