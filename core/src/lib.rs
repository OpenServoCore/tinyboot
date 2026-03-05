#![no_std]

pub mod hal;

mod boot;
mod log;

use boot::BootControl;
use hal::{Abi, Flash, Hal, Registry, Uart};

pub struct Core<UART, FLASH, REG, ABI>
where
    UART: Uart,
    FLASH: Flash,
    REG: Registry,
    ABI: Abi,
{
    hal: Hal<UART, FLASH, REG, ABI>,
}

impl<UART, FLASH, REG, ABI> Core<UART, FLASH, REG, ABI>
where
    UART: Uart,
    FLASH: Flash,
    REG: Registry,
    ABI: Abi,
{
    pub fn new(hal: Hal<UART, FLASH, REG, ABI>) -> Self {
        Core { hal }
    }

    pub fn run(&mut self) -> ! {
        log_info!("Bootloader started");

        let boot = BootControl::new(&mut self.hal.abi, &mut self.hal.reg);

        if boot.should_boot_app() {
            log_info!("Jumping to application");
            self.hal.abi.jump_to_app();
        }

        log_info!("Entering bootloader mode");

        loop {}
    }
}
