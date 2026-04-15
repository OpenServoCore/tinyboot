#[cfg_attr(flash_v0, path = "v0.rs")]
#[cfg_attr(flash_v1, path = "v1.rs")]
mod family;

pub use family::*;

/// Boot metadata address (ORIGIN of META region).
/// Defined by `__tb_meta_base` in tb-boot.x / tb-app.x.
#[inline(always)]
pub fn meta_addr() -> u32 {
    unsafe extern "C" {
        static __tb_meta_base: u8;
    }
    unsafe { &__tb_meta_base as *const u8 as u32 }
}

/// App region base address (ORIGIN of APP region).
/// Defined by `__tb_app_base` in tb-boot.x.
#[inline(always)]
pub fn app_base() -> u32 {
    unsafe extern "C" {
        static __tb_app_base: u8;
    }
    unsafe { &__tb_app_base as *const u8 as u32 }
}
