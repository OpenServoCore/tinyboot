//! User-flash hand-off: reset APB2 peripherals, jump to `__tb_app_entry`.
//!
//! The symbol is loaded inside `execute` (not at construction) so the app —
//! which never hands off to itself — doesn't pull in an unresolved reference;
//! LTO drops the whole method from the app binary.

use crate::hal::{pfic, rcc};

pub struct UserHandOff;

impl UserHandOff {
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn execute(&mut self) -> ! {
        unsafe extern "C" {
            static __tb_app_entry: u8;
        }
        let app_entry = unsafe { &__tb_app_entry as *const u8 as u32 };
        rcc::reset_apb2();
        pfic::jump(app_entry)
    }
}
