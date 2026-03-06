use smol_boot::{hal::Abi, log_info};

use crate::hal::common::*;

pub(crate) struct Ch32Abi;

impl Ch32Abi {
    pub fn new() -> Self {
        Ch32Abi {}
    }
}

impl Abi for Ch32Abi {
    fn app_magic(&self) -> u32 {
        unsafe { core::ptr::read_volatile(APP_PTR) }
    }

    fn app_flash_region(&self) -> (u32, u32) {
        (
            APP_BASE,
            APP_BASE + APP_SIZE as u32,
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
    unsafe { core::mem::transmute::<_, EntryPoint>(APP_PTR.add(1)) }
}
