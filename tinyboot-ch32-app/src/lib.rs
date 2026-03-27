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
            let ob = flash::META_OB_BASE;
            let state = unsafe { core::ptr::read_volatile(ob as *const u8) };
            if BootState::from_u8(state) != BootState::Validating {
                return;
            }
            // Read current meta, set state=Idle and reset trials
            let mut meta = [0xFFu8; 8];
            for (i, slot) in meta.iter_mut().enumerate() {
                *slot = unsafe { core::ptr::read_volatile((ob + i as u32 * 2) as *const u8) };
            }
            meta[0] = BootState::Idle as u8;
            meta[1] = 0xFF;
            // Read chip config, erase OB, rewrite
            let mut buf = [0xFFu8; 16];
            for (i, slot) in buf[..8].iter_mut().enumerate() {
                *slot = unsafe {
                    core::ptr::read_volatile((flash::OB_BASE + i as u32 * 2) as *const u8)
                };
            }
            buf[8..16].copy_from_slice(&meta);
            flash::unlock();
            iwdg::feed();
            flash::ob_erase();
            flash::ob_write(flash::OB_BASE, &buf);
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
/// Reads boot version from flash at `boot_base + boot_size - 2`.
/// Reads app version from the `__tinyboot_version` linker symbol.
pub fn new_app(
    boot_base: u32,
    boot_size: u32,
    app_size: u32,
    erase_size: u16,
) -> App<Ch32BootClient> {
    let boot_ver_addr = (boot_base + boot_size - 2) as *const u16;
    App::new(
        AppConfig {
            capacity: app_size,
            erase_size,
            boot_version: unsafe { boot_ver_addr.read_volatile() },
            app_version: tinyboot::tinyboot_version(),
        },
        Ch32BootClient,
    )
}
