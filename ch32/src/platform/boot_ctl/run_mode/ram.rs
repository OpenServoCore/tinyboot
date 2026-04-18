//! Run-mode persisted in a RAM magic word at `__tb_run_mode`.

use tinyboot_core::traits::RunMode;

const MAGIC: u32 = 0xB007_CAFE;

unsafe extern "C" {
    static mut __tb_run_mode: u32;
}

pub struct RamRunModeCtl;

impl RamRunModeCtl {
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn read(&self) -> RunMode {
        let v = unsafe { core::ptr::read_volatile(&raw const __tb_run_mode) };
        if v == MAGIC {
            RunMode::Service
        } else {
            RunMode::HandOff
        }
    }

    #[inline(always)]
    pub fn write(&mut self, mode: RunMode) {
        let val = if mode == RunMode::Service { MAGIC } else { 0 };
        unsafe { core::ptr::write_volatile(&raw mut __tb_run_mode, val) };
    }
}
