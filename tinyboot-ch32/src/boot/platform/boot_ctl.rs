use tinyboot::traits::BootCtl as TBBootCtl;

use crate::hal::{flash, pfic};

#[derive(Default)]
pub struct BootCtl;

impl TBBootCtl for BootCtl {
    fn is_boot_requested(&self) -> bool {
        flash::is_boot_mode()
    }

    fn clear_boot_request(&mut self) {
        flash::set_boot_mode(false);
    }

    fn system_reset(&mut self) -> ! {
        pfic::system_reset();
    }
}
