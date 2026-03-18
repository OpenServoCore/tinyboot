use tinyboot::traits::BootCtl as TBBootCtl;

use tinyboot_ch32_hal::pfic;

pub struct BootCtlConfig {
    /// App entry point address (execution alias, not FPEC address).
    /// Only used for user-flash bootloaders that must jump to the app.
    #[cfg(not(feature = "system-flash"))]
    pub app_entry: u32,
}

pub struct BootCtl {
    #[cfg(not(feature = "system-flash"))]
    app_entry: u32,
}

impl BootCtl {
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

    fn clear_boot_request(&mut self) {
        #[cfg(feature = "system-flash")]
        tinyboot_ch32_hal::flash::set_boot_mode(false);
        #[cfg(not(feature = "system-flash"))]
        tinyboot_ch32_hal::boot_request::set_boot_request(false);
    }

    fn system_reset(&mut self) -> ! {
        pfic::system_reset();
    }

    fn boot_app(&mut self) -> ! {
        self.clear_boot_request();
        #[cfg(feature = "system-flash")]
        self.system_reset();
        #[cfg(not(feature = "system-flash"))]
        pfic::jump(self.app_entry)
    }
}
