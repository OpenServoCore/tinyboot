#![no_std]
#![no_main]

use qingke_rt::entry;

#[cfg(not(feature = "log"))]
use panic_halt as _;

#[cfg(feature = "log")]
use defmt_rtt as _;

#[cfg(feature = "log")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panic");
    loop {}
}

use ch32_iap_core::BootControl;

#[entry]
fn main() -> ! {
    #[cfg(feature = "log")]
    defmt::info!("Bootloader started");

    let boot = BootControl::read();

    if boot.should_boot_app() {
        #[cfg(feature = "log")]
        defmt::info!("Jumping to application");
        unsafe { boot.jump_to_app() }
    }

    #[cfg(feature = "log")]
    defmt::info!("Entering bootloader mode");

    loop {}
}
