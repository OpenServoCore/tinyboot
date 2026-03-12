#![no_std]

pub(crate) mod common;
pub(crate) mod hal;
mod platform;
mod rt;

use tiny_boot::{Core, traits::Platform};

use platform::{BaudRate, BootCtl, BootMetaStore, Duplex, Storage, Usart, UsartConfig};

type Ch32Core = Core<Usart, Storage, BootMetaStore, BootCtl>;

pub struct Bootloader {
    core: Ch32Core,
}

impl Default for Bootloader {
    fn default() -> Self {
        // TODO: Move away from Default trait and implement BootloaderConfig
        let config = UsartConfig {
            duplex: Duplex::Full,
            baud: BaudRate::B115200,
            // TODO: compute pclk from RCC registers instead of hardcoding
            pclk: 8_000_000,
        };
        let transport = Usart::new(ch32_metapac::USART1, &config);
        let storage = Storage::new(ch32_metapac::FLASH);
        let ctl = BootCtl::new();
        let boot_meta = BootMetaStore::new(ch32_metapac::FLASH);
        let platform = Platform::new(transport, storage, boot_meta, ctl);
        let core = Core::new(platform);

        Bootloader { core }
    }
}

impl Bootloader {
    pub fn run(&mut self) -> ! {
        self.core.run();
    }
}
