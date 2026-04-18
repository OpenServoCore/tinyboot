//! System-flash hand-off: software reset; ROM dispatches on BOOT_MODE.

use crate::hal::pfic;

pub struct SystemHandOff;

impl SystemHandOff {
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn execute(&mut self) -> ! {
        pfic::software_reset()
    }
}
