#![no_std]

mod hal;

use smol_boot::{Core, hal::Hal};

use hal::{Abi, Flash, Registry};
use hal::transport::usart::Usart;

type Ch32Core = Core<Usart, Flash, Registry, Abi>;

pub struct Bootloader {
    core: Ch32Core,
}

impl Default for Bootloader {
    fn default() -> Self {
        let transport = Usart::new(ch32_metapac::USART1);
        let flash = Flash::new(ch32_metapac::FLASH);
        let abi = Abi::new();
        let reg = Registry::new();
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
