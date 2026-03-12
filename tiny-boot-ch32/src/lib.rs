#![no_std]

pub mod hal;

use tiny_boot::{Core, hal::Hal};

use hal::transport::usart::{BaudRate, Duplex, Usart, UsartConfig};
use hal::{Abi, Flash, Registry};

type Ch32Core = Core<Usart, Flash, Registry, Abi>;

pub struct Bootloader {
    core: Ch32Core,
}

impl Default for Bootloader {
    fn default() -> Self {
        // TODO: Move away from Default trait and implement BootloaderConfig
        let config = UsartConfig {
            duplex: Duplex::Full,
            baud: BaudRate::B115200,
            pclk: 48_000_000,
        };
        let transport = Usart::new(ch32_metapac::USART1, &config);
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
