//! CH32 app-side tinyboot client.

use crate::hal::flash;
use crate::platform::BootMetaStore;

pub use crate::hal::Pin;
pub use crate::hal::gpio::Level;
pub use crate::platform::BootCtl;
pub use tinyboot_core::BaudRate;
pub use tinyboot_core::app::{App, AppConfig};
pub use tinyboot_core::traits;
pub use tinyboot_core::{app_version, pkg_version};

/// Create an [`App`] configured for CH32 hardware.
pub fn new_app(ctl: BootCtl) -> App<BootCtl, BootMetaStore> {
    unsafe extern "C" {
        static __tb_boot_tail: tinyboot_core::ImageTail;
        static __tb_app_capacity: u8;
    }
    let boot_tail = &raw const __tb_boot_tail;
    let app_capacity = unsafe { &__tb_app_capacity as *const u8 as u32 };
    App::new(
        AppConfig {
            capacity: app_capacity,
            erase_size: flash::PAGE_SIZE as u16,
            boot_version: unsafe { (*boot_tail).version },
            boot_compat: unsafe { (*boot_tail).compat },
            app_version: tinyboot_core::tinyboot_version(),
        },
        ctl,
        BootMetaStore::default(),
    )
}
