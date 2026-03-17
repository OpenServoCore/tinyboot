#![no_std]
#![no_main]

use panic_halt as _;

#[cfg(feature = "defmt")]
use defmt_rtt as _;

use tinyboot::traits::BootClient;
use tinyboot_ch32::app;

#[qingke_rt::entry]
fn main() -> ! {
    let mut client = app::BootClient::new(app::BootClientConfig {
        meta_base: 0x1FFF_FCC0,
    });
    client.confirm();

    #[cfg(feature = "defmt")]
    defmt::info!("Hello from App B!");

    loop {}
}
