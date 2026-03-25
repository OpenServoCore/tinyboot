use tinyboot::traits::BootMode;
use tinyboot::traits::boot::BootCtl as TBBootCtl;

use tinyboot_ch32_hal::pfic;

/// Boot control configuration.
pub struct BootCtlConfig {
    /// App entry point address (execution alias, not FPEC address).
    /// Only used for user-flash bootloaders that must jump to the app.
    #[cfg(not(feature = "system-flash"))]
    pub app_entry: u32,
}

/// CH32 boot control (reset, boot mode selection).
pub struct BootCtl {
    #[cfg(not(feature = "system-flash"))]
    app_entry: u32,
}

impl BootCtl {
    /// Create boot control from configuration.
    #[inline(always)]
    pub fn new(_config: BootCtlConfig) -> Self {
        Self {
            #[cfg(not(feature = "system-flash"))]
            app_entry: _config.app_entry,
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
