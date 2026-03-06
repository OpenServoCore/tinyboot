#![no_std]
#![no_main]

use panic_halt as _;

use smol_boot_ch32::Bootloader;
use qingke_rt::entry;

#[entry]
fn main() -> ! {
    Bootloader::default().run();
}
