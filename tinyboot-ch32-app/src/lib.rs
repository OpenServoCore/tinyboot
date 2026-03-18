#![no_std]

use tinyboot::traits::BootClient as TBBootClient;

use tinyboot_ch32_hal::pfic;

pub struct BootClientConfig {
    pub meta_base: u32,
}

pub struct BootClient {
    #[cfg(feature = "trial-boot")]
    meta_base: u32,
}

impl BootClient {
    pub fn new(_config: BootClientConfig) -> Self {
        BootClient {
            #[cfg(feature = "trial-boot")]
            meta_base: _config.meta_base,
        }
    }
}

impl TBBootClient for BootClient {
    fn confirm(&mut self) {
        #[cfg(feature = "trial-boot")]
        {
            use tinyboot::traits::{BootMeta, BootState};
            use tinyboot_ch32_hal::flash::FlashWriter;

            const STATE_OFFSET: u32 = 0;

            critical_section::with(|_| {
                let meta: BootMeta =
                    unsafe { core::ptr::read_volatile(self.meta_base as *const BootMeta) };
                if meta.boot_state() != BootState::Validating {
                    return;
                }
                let next = meta.state & (meta.state >> 1);
                #[cfg(feature = "system-flash")]
                let writer = FlashWriter::system();
                #[cfg(not(feature = "system-flash"))]
                let writer = FlashWriter::standard();
                writer.write_halfword(self.meta_base + STATE_OFFSET, next);
            });
        }
    }

    fn request_update(&mut self) -> ! {
        critical_section::with(|_| {
            #[cfg(feature = "system-flash")]
            tinyboot_ch32_hal::flash::set_boot_mode(true);
            #[cfg(not(feature = "system-flash"))]
            tinyboot_ch32_hal::boot_request::set_boot_request(true);
        });
        pfic::system_reset()
    }
}
