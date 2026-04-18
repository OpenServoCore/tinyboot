//! Run-mode inferred from the flash BOOT_MODE register (V003 + system-flash).
//!
//! The register doubles as boot-source latch and run-mode store. Writes are
//! owned by `ModeBootSrcCtl` to avoid redundant flash unlock cycles; `write`
//! here is a no-op. `read` reports the current register state.

use tinyboot_core::traits::RunMode;

use crate::hal::flash;

pub struct ModeRunModeCtl;

impl ModeRunModeCtl {
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn read(&self) -> RunMode {
        if flash::boot_mode() {
            RunMode::Service
        } else {
            RunMode::HandOff
        }
    }

    #[inline(always)]
    pub fn write(&mut self, _mode: RunMode) {
        // ModeBootSrcCtl owns the register write.
    }
}
