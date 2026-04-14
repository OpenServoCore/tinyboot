use tinyboot::traits::BootMode;
use tinyboot::traits::boot::BootCtl as TBBootCtl;

use tinyboot_ch32_hal::pfic;

/// CH32 boot control (reset, boot mode selection).
///
/// In user-flash mode, caches the app entry point from `__tb_app_entry` linker symbol.
pub struct BootCtl {
    #[cfg(not(feature = "system-flash"))]
    app_entry: u32,
}

impl Default for BootCtl {
    /// In user-flash mode, reads `__tb_app_entry` linker symbol for the app entry point.
    #[inline(always)]
    fn default() -> Self {
        Self {
            #[cfg(not(feature = "system-flash"))]
            app_entry: {
                unsafe extern "C" {
                    static __tb_app_entry: u8;
                }
                unsafe { &__tb_app_entry as *const u8 as u32 }
            },
        }
    }
}

impl TBBootCtl for BootCtl {
    fn is_boot_requested(&self) -> bool {
        #[cfg(feature = "system-flash")]
        {
            tinyboot_ch32_hal::flash::is_boot_mode()
        }
        #[cfg(not(feature = "system-flash"))]
        {
            tinyboot_ch32_hal::boot_request::is_boot_requested()
        }
    }

    fn system_reset(&mut self, mode: BootMode) -> ! {
        let bootloader = mode == BootMode::Bootloader;
        #[cfg(feature = "system-flash")]
        {
            tinyboot_ch32_hal::flash::set_boot_mode(bootloader);
            pfic::system_reset()
        }
        #[cfg(not(feature = "system-flash"))]
        {
            tinyboot_ch32_hal::boot_request::set_boot_request(bootloader);
            if bootloader {
                pfic::system_reset()
            } else {
                tinyboot_ch32_hal::flash::lock();
                tinyboot_ch32_hal::rcc::reset_apb2();
                pfic::jump(self.app_entry)
            }
        }
    }
}
