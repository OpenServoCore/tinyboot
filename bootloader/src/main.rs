#![no_std]
#![no_main]

#[cfg(not(rtt_log))]
use panic_halt as _;

#[cfg(rtt_log)]
use defmt_rtt as _;

#[cfg(rtt_log)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    log_error!("panic");
    loop {}
}

use ch32_iap_bootloader::Bootloader;
use qingke_rt::entry;

#[entry]
fn main() -> ! {
    Bootloader::default().run();
}
