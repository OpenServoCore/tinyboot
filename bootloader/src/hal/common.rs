unsafe extern "C" {
    static __APP_ADDR: u8;
    static __APP_SIZE: u8;
}

pub(crate) fn app_flash_addr() -> *const u32 {
    &raw const __APP_ADDR as *const u32
}

pub(crate) fn app_flash_start() -> u32 {
    &raw const __APP_ADDR as u32
}

pub(crate) fn app_flash_size() -> usize {
    &raw const __APP_SIZE as usize
}
