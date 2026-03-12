use tiny_boot::{traits::BootCtl as TBBootCtl, log_info};

use crate::common::*;
use crate::hal::pfic;

pub(crate) struct BootCtl;

impl BootCtl {
    pub fn new() -> Self {
        BootCtl {}
    }
}

impl TBBootCtl for BootCtl {
    fn jump_to_app(&self) -> ! {
        log_info!("Booting Application...");
        let ep = entry_point();
        unsafe { ep() };
    }

    fn system_reset(&mut self) -> ! {
        log_info!("Resetting...");
        pfic::system_reset(&ch32_metapac::PFIC);
    }
}

type EntryPoint = unsafe extern "C" fn() -> !;

fn entry_point() -> EntryPoint {
    unsafe { core::mem::transmute::<_, EntryPoint>(APP_PTR) }
}
