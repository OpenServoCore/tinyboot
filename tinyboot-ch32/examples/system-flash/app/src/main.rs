//! Example application for the system-flash bootloader.
//!
//! This app occupies the full 16KB user flash. Boot metadata is in system
//! flash, so the app's `meta_base` must point there.
//!
//! To request a firmware update from the app, call `client.request_update()`.
//! With the `system-flash` feature, this sets the hardware BOOT_MODE register
//! and triggers a soft reset back into the bootloader.

#![no_std]
#![no_main]

use panic_halt as _;
use defmt_rtt as _;

use tinyboot::traits::BootClient;
use tinyboot_ch32::app;

/// Boot metadata location in system flash (must match bootloader's META_BASE).
const META_BASE: u32 = 0x1FFF_FCC0;

#[qingke_rt::entry]
fn main() -> ! {
    // Confirm successful boot to the bootloader's trial-boot FSM.
    // This advances the boot state from Validating -> Confirmed.
    // Safe to call unconditionally; it's a no-op if not in Validating state.
    let mut client = app::BootClient::new(app::BootClientConfig {
        meta_base: META_BASE,
    });
    client.confirm();

    defmt::info!("Hello from app!");

    loop {}
}
