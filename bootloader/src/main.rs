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

use ch32_iap_core::{BootMode, determine_boot_mode, jump_to_application};

#[entry]
fn main() -> ! {
    #[cfg(feature = "log")]
    defmt::info!("Bootloader started");

    match determine_boot_mode() {
        BootMode::Bootloader => {
            #[cfg(feature = "log")]
            defmt::info!("Entering bootloader mode");
        }
        BootMode::Application => {
            #[cfg(feature = "log")]
            defmt::info!("Jumping to application");
            jump_to_application();
        }
    }

    loop {}
}
