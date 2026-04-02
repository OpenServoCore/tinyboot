//! Example application for the tinyboot bootloader.
//!
//! - Timer interrupt blinks an LED every second
//! - Main loop listens on USART1 for tinyboot commands,
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

#[cfg(feature = "user-flash")]
use defmt_rtt as _;

tinyboot_ch32_app::app_version!();

#[cfg(feature = "user-flash")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panic");
    loop {}
}

#[cfg(feature = "system-flash")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// --- Flash layout (must match bootloader) ---

#[cfg(feature = "system-flash")]
const BOOT_BASE: u32 = 0x1FFF_F000;

#[cfg(feature = "user-flash")]
const BOOT_BASE: u32 = 0x0800_0000;

#[cfg(feature = "user-flash")]
const BOOT_SIZE: u32 = 8 * 1024;

// CH32V003: 1920-byte system flash, 16KB user flash, 64-byte erase pages
#[cfg(any(
    feature = "ch32v003f4p6",
    feature = "ch32v003a4m6",
    feature = "ch32v003f4u6",
    feature = "ch32v003j4m6",
))]
mod chip {
    #[cfg(feature = "system-flash")]
    pub const BOOT_SIZE: u32 = 1920;
    pub const APP_SIZE: u32 = if cfg!(feature = "system-flash") {
        16 * 1024
    } else {
        8 * 1024
    };
    pub const ERASE_SIZE: u16 = 64;
}

// CH32V103C6: 2048-byte system flash, 32KB user flash, 128-byte erase pages
#[cfg(feature = "ch32v103c6t6")]
mod chip {
    #[cfg(feature = "system-flash")]
    pub const BOOT_SIZE: u32 = 2048;
    pub const APP_SIZE: u32 = if cfg!(feature = "system-flash") {
        32 * 1024
    } else {
        24 * 1024
    };
    pub const ERASE_SIZE: u16 = 128;
}

// CH32V103C8/R8: 2048-byte system flash, 64KB user flash, 128-byte erase pages
#[cfg(any(
    feature = "ch32v103c8t6",
    feature = "ch32v103c8u6",
    feature = "ch32v103r8t6",
))]
mod chip {
    #[cfg(feature = "system-flash")]
    pub const BOOT_SIZE: u32 = 2048;
    pub const APP_SIZE: u32 = if cfg!(feature = "system-flash") {
        64 * 1024
    } else {
        56 * 1024
    };
    pub const ERASE_SIZE: u16 = 128;
}

#[cfg(feature = "system-flash")]
const BOOT_SIZE: u32 = chip::BOOT_SIZE;

type Shared<T> = Mutex<RefCell<Option<T>>>;
static LED: Shared<Output<'static>> = Mutex::new(RefCell::new(None));

#[qingke_rt::interrupt]
fn TIM2() {
    pac::TIM2.intfr().modify(|w| w.set_uif(false));
    critical_section::with(|cs| {
        if let Some(ref mut led) = *LED.borrow_ref_mut(cs) {
            led.toggle();
            #[cfg(feature = "user-flash")]
            if led.is_set_high() {
                defmt::info!("LED on");
            } else {
                defmt::info!("LED off");
            }
        }
    });
}

#[cfg(feature = "user-flash")]
tinyboot_ch32_app::fix_mtvec!();

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
    let mut uart_config = usart::Config::default();
    uart_config.baudrate = 115200;

    // CH32V003 Remap0: TX=PD5, RX=PD6
    #[cfg(any(
        feature = "ch32v003f4p6",
        feature = "ch32v003a4m6",
        feature = "ch32v003f4u6",
        feature = "ch32v003j4m6",
    ))]
    let uart = Uart::new_blocking::<0>(p.USART1, p.PD6, p.PD5, uart_config).unwrap();

    // CH32V103 Remap0: TX=PA9, RX=PA10
    #[cfg(any(
        feature = "ch32v103c6t6",
        feature = "ch32v103c8t6",
        feature = "ch32v103c8u6",
        feature = "ch32v103r8t6",
    ))]
    let uart = Uart::new_blocking::<0>(p.USART1, p.PA10, p.PA9, uart_config).unwrap();
    let (tx, rx) = uart.split();
    let mut rx = transport::Rx(rx);
    let mut tx = transport::Tx(tx);

    // Tinyboot app client
    // V103 system-flash (boot pin): configure GPIO pin driving BOOT0 circuit
    #[cfg(all(
        feature = "system-flash",
        any(
            feature = "ch32v103c6t6",
            feature = "ch32v103c8t6",
            feature = "ch32v103c8u6",
            feature = "ch32v103r8t6",
        )
    ))]
    let boot_ctl_config = tinyboot_ch32_app::BootCtlConfig {
        pin: tinyboot_ch32_app::Pin::PA0,
        active_high: true,
    };

    // All other cases: unit config (no boot pin, or user-flash)
    #[cfg(not(all(
        feature = "system-flash",
        any(
            feature = "ch32v103c6t6",
            feature = "ch32v103c8t6",
            feature = "ch32v103c8u6",
            feature = "ch32v103r8t6",
        )
    )))]
    let boot_ctl_config = tinyboot_ch32_app::BootCtlConfig;

    let mut app = tinyboot_ch32_app::new_app(
        BOOT_BASE,
        BOOT_SIZE,
        chip::APP_SIZE,
        chip::ERASE_SIZE,
        boot_ctl_config,
    );
    app.confirm();

    #[cfg(feature = "user-flash")]
    defmt::info!("Boot confirmed, app ready.");

    loop {
        app.poll(&mut rx, &mut tx);
    }
}
