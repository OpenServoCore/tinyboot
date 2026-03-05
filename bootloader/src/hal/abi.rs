use ch32_iap_core::{hal::Abi, log_info};

use crate::hal::common::*;

pub(crate) struct Ch32Abi;

impl Ch32Abi {
    pub fn new() -> Self {
        Ch32Abi {}
    }
}

impl Abi for Ch32Abi {
    fn app_magic(&self) -> u32 {
        unsafe { core::ptr::read_volatile(app_flash_addr()) }
    }

    fn app_flash_region(&self) -> (u32, u32) {
        (
            app_flash_start(),
            app_flash_start() + app_flash_size() as u32,
        )
    }

    fn jump_to_app(&self) -> ! {
        log_info!("Booting Application...");
        let ep = entry_point();
        unsafe { ep() };
    }

    fn system_reset(&mut self) -> ! {
        log_info!("Resetting...");
        todo!()
    }
}

type EntryPoint = unsafe extern "C" fn() -> !;

fn entry_point() -> EntryPoint {
    unsafe { core::mem::transmute::<_, EntryPoint>(app_flash_addr().add(1)) }
}
