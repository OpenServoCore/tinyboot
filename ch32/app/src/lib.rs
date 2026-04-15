#![no_std]

use tinyboot::traits::BootState;
use tinyboot::traits::app::BootClient as TBBootClient;
use tinyboot_ch32_hal::{boot_request, flash, iwdg, pfic};

// Re-exports so apps only need this one crate.
pub use boot_request::Config as BootCtlConfig;
pub use tinyboot::app::{App, AppConfig};
pub use tinyboot::traits::app as traits;
pub use tinyboot::{app_version, pkg_version};
pub use tinyboot_ch32_hal::Pin;

/// CH32 boot client implementation.
pub struct BootClient {
    config: boot_request::Config,
}

impl TBBootClient for BootClient {
    fn confirm(&mut self) {
        critical_section::with(|_| {
            let addr = flash::meta_addr();
            let mut meta = unsafe { core::ptr::read_volatile(addr as *const [u8; 8]) };
            if BootState::from_u8(meta[0]) != BootState::Validating {
                return;
            }

            meta[0] = BootState::Idle as u8;
            meta[1] = 0xFF;

            flash::unlock();
            iwdg::feed();
            flash::erase(addr);
            flash::write(addr, &meta);
            flash::lock();
        });
    }

    fn request_update(&mut self) {
        critical_section::with(|_| {
            boot_request::set_boot_request(&self.config, true);
        });
        // Allow time for external boot mode circuit (RC) to settle.
        for _ in 0..8000u16 {
            core::hint::spin_loop();
        }
    }

    fn system_reset(&mut self) -> ! {
        pfic::system_reset()
    }
}

/// Create an [`App`] configured for CH32 hardware.
///
/// Reads boot version from `__tb_boot_version_addr`, app capacity from
/// `__tb_app_capacity`, and erase size from `flash::PAGE_SIZE`.
pub fn new_app(boot_ctl: boot_request::Config) -> App<BootClient> {
    boot_request::init(&boot_ctl);
    unsafe extern "C" {
        static __tb_boot_version_addr: u8;
        static __tb_app_capacity: u8;
    }
    let boot_ver_addr = unsafe { &__tb_boot_version_addr as *const u8 as *const u16 };
    let app_capacity = unsafe { &__tb_app_capacity as *const u8 as u32 };
    App::new(
        AppConfig {
            capacity: app_capacity,
            erase_size: flash::PAGE_SIZE as u16,
            boot_version: unsafe { boot_ver_addr.read_volatile() },
            app_version: tinyboot::tinyboot_version(),
        },
        BootClient { config: boot_ctl },
    )
}
