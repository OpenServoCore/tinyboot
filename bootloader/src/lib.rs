#![no_std]

mod hal;

use smol_boot::{Core, hal::Hal};

use hal::{Ch32Abi, Ch32Flash, Ch32Registry, Ch32Transport};

type Ch32Core = Core<Ch32Transport, Ch32Flash, Ch32Registry, Ch32Abi>;

pub struct Bootloader {
    core: Ch32Core,
}

impl Default for Bootloader {
    fn default() -> Self {
        let transport = Ch32Transport::new();
        let flash = Ch32Flash::new();
        let abi = Ch32Abi::new();
        let reg = Ch32Registry::new();
        let hal = Hal::new(transport, flash, reg, abi);
        let core = Core::new(hal);

        Bootloader { core }
    }
}

impl Bootloader {
    pub fn run(&mut self) -> ! {
        self.core.run();
    }
}
