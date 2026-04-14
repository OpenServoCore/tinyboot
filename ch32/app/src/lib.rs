#![no_std]

use tinyboot::traits::BootState;
use tinyboot::traits::app::BootClient as TBBootClient;
use tinyboot_ch32_hal::{flash, iwdg, pfic};

// Re-exports so apps only need this one crate.
pub use tinyboot::app::{App, AppConfig};
pub use tinyboot::traits::app as traits;
pub use tinyboot::{app_version, pkg_version};

#[doc(hidden)]
pub use qingke;

/// Fix `mtvec` for apps linked at a non-zero flash address (user-flash bootloader).
///
/// `qingke-rt` hardcodes `mtvec = 0x0` in its `_setup_interrupts`. This macro
/// generates a linker `--wrap` override that calls the original setup, then
/// rewrites `mtvec` to the actual vector table base.
///
/// Not needed for system-flash bootloaders (app starts at 0x0).
///
/// Place at module scope alongside [`app_version!`]. Requires
/// `--wrap=_setup_interrupts` in `build.rs`:
///
/// ```rust,ignore
/// // build.rs
/// println!("cargo:rustc-link-arg=--wrap=_setup_interrupts");
/// ```
#[cfg(not(feature = "system-flash"))]
#[macro_export]
macro_rules! fix_mtvec {
    () => {
        #[unsafe(export_name = "__wrap__setup_interrupts")]
        unsafe extern "C" fn _tinyboot_setup_interrupts() {
            use $crate::qingke::register::mtvec::{self, TrapMode};

            unsafe extern "C" {
                fn __real__setup_interrupts();
                fn _start();
            }
            unsafe {
                __real__setup_interrupts();
                mtvec::write(_start as *const () as usize, TrapMode::VectoredAddress);
            }
        }
    };
}

/// CH32 boot client implementation.
pub struct Ch32BootClient;

impl TBBootClient for Ch32BootClient {
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
            #[cfg(feature = "system-flash")]
            flash::set_boot_mode(true);
            #[cfg(not(feature = "system-flash"))]
            tinyboot_ch32_hal::boot_request::set_boot_request(true);
        });
    }

    fn system_reset(&mut self) -> ! {
        pfic::system_reset()
    }
}

/// Create an [`App`] configured for CH32 hardware.
///
/// Reads boot version from `__tb_boot_version_addr`, app capacity from
/// `__tb_app_capacity`, and erase size from `flash::PAGE_SIZE`.
pub fn new_app() -> App<Ch32BootClient> {
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
        Ch32BootClient,
    )
}
