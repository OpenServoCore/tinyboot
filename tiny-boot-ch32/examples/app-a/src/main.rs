#![no_std]
#![no_main]

use panic_halt as _;

#[cfg(feature = "defmt")]
use defmt_rtt as _;

use tiny_boot::traits::BootClient;
use tiny_boot_ch32::app;

#[qingke_rt::entry]
fn main() -> ! {
    let mut client = app::BootClient::default();
    client.confirm();

    #[cfg(feature = "defmt")]
    defmt::info!("Hello from App A!");

    loop {}
}
