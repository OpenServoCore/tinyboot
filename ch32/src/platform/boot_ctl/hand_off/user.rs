//! User-flash hand-off: reset APB2 peripherals, then jump to the app's reset
//! vector at the `__tb_app_entry` linker symbol.
//!
//! The symbol is read inside [`execute`](Self::execute) (not at construction)
//! so the app binary — which never hands off to itself — does not pull in an
//! unresolved reference. In the app, LTO drops `execute` entirely.

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
