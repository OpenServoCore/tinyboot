//! Boot source via the flash BOOT_MODE register (V003 + system-flash).
//! BOOT_MODE=1 → system flash; 0 → user flash. Sole writer of the register;
//! paired with the read-only `ModeRunModeCtl`.

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
