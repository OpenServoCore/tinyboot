#![no_std]

mod hal;

use ch32_iap_core::{Core, hal::Hal};

use hal::{Ch32Abi, Ch32Flash, Ch32Registry, Ch32Uart};

type Ch32Core = Core<Ch32Uart, Ch32Flash, Ch32Registry, Ch32Abi>;

pub struct Bootloader {
    core: Ch32Core,
}

impl Default for Bootloader {
    fn default() -> Self {
        let uart = Ch32Uart::new();
        let flash = Ch32Flash::new();
        let abi = Ch32Abi::new();
        let reg = Ch32Registry::new();
        let hal = Hal::new(uart, flash, reg, abi);
        let core = Core::new(hal);

        Bootloader { core }
    }
}

impl Bootloader {
    pub fn run(&mut self) -> ! {
        self.core.run();
    }
}
