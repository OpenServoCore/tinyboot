use tinyboot::traits::{BootClient as TBBootClient, BootMeta, BootState};

use crate::hal::flash::{self, FlashWriter};
use crate::hal::pfic;

const STATE_OFFSET: u32 = 0;

pub struct BootClientConfig {
    pub meta_base: u32,
}

pub struct BootClient {
    meta_base: u32,
}

impl BootClient {
    pub fn new(config: BootClientConfig) -> Self {
        BootClient {
            meta_base: config.meta_base,
        }
    }

    fn meta_ptr(&self) -> *const BootMeta {
        self.meta_base as *const BootMeta
    }
}

impl TBBootClient for BootClient {
    fn confirm(&mut self) {
        critical_section::with(|_| {
            let meta: BootMeta = unsafe { core::ptr::read_volatile(self.meta_ptr()) };
            if meta.boot_state() != BootState::Validating {
                return;
            }
            let next = meta.state & (meta.state >> 1);
            let writer = FlashWriter::standard();
            writer.write_halfword(self.meta_base + STATE_OFFSET, next);
        });
    }

    fn request_update(&mut self) -> ! {
        critical_section::with(|_| {
            flash::set_boot_mode(true);
        });
        pfic::system_reset();
    }
}
