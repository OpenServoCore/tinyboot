//! Boot source latched in the flash BOOT_MODE register (V003 + system-flash).
//!
//! BOOT_MODE=1 keeps the ROM in the system-flash bootloader; BOOT_MODE=0
//! sends it to user flash. Sole writer of the register on this target;
//! paired with [`super::super::run_mode::ModeRunModeCtl`] (read-only).

use super::BootSrc;
use crate::hal::flash;

pub struct ModeBootSrcCtl;

impl ModeBootSrcCtl {
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn set(&mut self, src: BootSrc) {
        flash::set_boot_mode(src == BootSrc::SystemFlash);
    }
}
