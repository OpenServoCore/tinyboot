//! CH32 app-side tinyboot client.

use crate::hal::flash;
use crate::platform::BootMetaStore;

pub use crate::hal::Pin;
pub use crate::hal::esig::uid as chip_uid;
pub use crate::hal::gpio::Level;
pub use crate::platform::BootCtl;
pub use tinyboot_core::BaudRate;
pub use tinyboot_core::addressing::{Addressable, Addressing, Unaddressed};
pub use tinyboot_core::app::{App, AppConfig};
pub use tinyboot_core::traits;
pub use tinyboot_core::{app_version, compat_key, pack_version, pkg_version};

/// Create an [`App`] configured for CH32 hardware.
pub fn new_app(ctl: BootCtl) -> App<BootCtl, BootMetaStore> {
    App::new(config(), ctl, BootMetaStore::default())
}

/// Create an addressable [`App`]: silent on a shared bus until a host
/// selects it by the chip's unique id. `wait_bytes(n)` waits about `n`
/// byte-times at the bus baud, to stagger `Select` replies.
pub fn new_addressable_app(
    ctl: BootCtl,
    wait_bytes: fn(u8),
) -> App<BootCtl, BootMetaStore, { tinyboot_core::frame::DEFAULT_PAYLOAD }, impl Addressing> {
    App::with_addressing(
        config(),
        ctl,
        BootMetaStore::default(),
        Addressable::new(chip_uid),
        wait_bytes,
    )
}

fn config() -> AppConfig {
    unsafe extern "C" {
        static __tb_boot_tail: tinyboot_core::ImageTail;
        static __tb_app_capacity: u8;
    }
    let boot_tail = &raw const __tb_boot_tail;
    let app_capacity = unsafe { &__tb_app_capacity as *const u8 as u32 };
    AppConfig {
        capacity: app_capacity,
        erase_size: flash::PAGE_SIZE as u16,
        boot_version: unsafe { (*boot_tail).version },
        boot_compat: unsafe { (*boot_tail).compat },
        app_version: tinyboot_core::tinyboot_version(),
    }
}
