use tiny_boot::{hal::Abi as SBAbi, log_info};

use crate::hal::common::*;

pub(crate) struct Abi;

impl Abi {
    pub fn new() -> Self {
        Abi {}
    }
}

impl SBAbi for Abi {
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
