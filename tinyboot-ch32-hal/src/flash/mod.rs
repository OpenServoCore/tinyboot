#[cfg_attr(flash_v0, path = "v0.rs")]
mod family;

pub use family::*;

/// Boot metadata address in programming space (0x0800xxxx).
/// Defined by `__tinyboot_meta_start` in memory.x.
#[inline(always)]
pub fn meta_addr() -> u32 {
    unsafe extern "C" {
        static __tinyboot_meta_start: u8;
    }
    unsafe { &__tinyboot_meta_start as *const u8 as u32 }
}
