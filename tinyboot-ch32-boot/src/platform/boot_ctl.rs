use tinyboot::traits::BootMode;
use tinyboot::traits::boot::BootCtl as TBBootCtl;

use tinyboot_ch32_hal::{boot_request, pfic};

/// CH32 boot control (reset, boot mode selection).
pub struct BootCtl {
    config: boot_request::Config,
    #[cfg(not(feature = "system-flash"))]
    app_entry: u32,
}

impl BootCtl {
    /// Create boot control for system-flash bootloaders.
    #[cfg(feature = "system-flash")]
    #[inline(always)]
    pub fn new(config: boot_request::Config) -> Self {
        boot_request::init(&config);
        Self { config }
    }

    /// Create boot control for user-flash bootloaders.
    ///
    /// `app_entry` is the execution-alias address of the application.
    #[cfg(not(feature = "system-flash"))]
    #[inline(always)]
    pub fn new(config: boot_request::Config, app_entry: u32) -> Self {
        boot_request::init(&config);
        Self { config, app_entry }
    }
}

impl TBBootCtl for BootCtl {
    fn is_boot_requested(&self) -> bool {
        boot_request::is_boot_requested()
    }

    fn system_reset(&mut self, mode: BootMode) -> ! {
        let bootloader = mode == BootMode::Bootloader;
        boot_request::set_boot_request(&self.config, bootloader);
        #[cfg(not(feature = "system-flash"))]
        if !bootloader {
            tinyboot_ch32_hal::flash::lock();
            tinyboot_ch32_hal::rcc::reset_apb2();
            pfic::jump(self.app_entry)
        }
        pfic::system_reset()
    }
}
