#![no_std]

pub mod hal;

mod boot;
mod log;

use boot::BootControl;
use hal::{Abi, Flash, Hal, Registry, Transport};

pub struct Core<T, F, R, A>
where
    T: Transport,
    F: Flash,
    R: Registry,
    A: Abi,
{
    hal: Hal<T, F, R, A>,
}

impl<T, F, R, A> Core<T, F, R, A>
where
    T: Transport,
    F: Flash,
    R: Registry,
    A: Abi,
{
    pub fn new(hal: Hal<T, F, R, A>) -> Self {
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
