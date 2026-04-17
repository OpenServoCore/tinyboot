//! Run-mode inferred from the flash BOOT_MODE register bit.
//!
//! V003 + system-flash only. The bootloader is reached only when the ROM saw
//! BOOT_MODE=1, so the register doubles as the boot-source latch
//! ([`super::super::boot_src::ModeBootSrcCtl`]) and the run-mode store.
//! To avoid a redundant flash-unlock+write in [`write`](Self::write), the
//! physical register write is owned by `ModeBootSrcCtl::set`; this type's
//! `write` is a no-op. `read` still reports the current register state.

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
        // No-op: ModeBootSrcCtl owns the BOOT_MODE register write.
    }
}
