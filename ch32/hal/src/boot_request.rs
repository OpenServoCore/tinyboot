const BOOT_REQUEST_MAGIC: u32 = 0xB007_CAFE;

unsafe extern "C" {
    static mut __tb_boot_request: u32;
}

pub fn is_boot_requested() -> bool {
    unsafe { core::ptr::read_volatile(&raw const __tb_boot_request) == BOOT_REQUEST_MAGIC }
}

pub fn set_boot_request(request: bool) {
    let val = if request { BOOT_REQUEST_MAGIC } else { 0 };
    unsafe { core::ptr::write_volatile(&raw mut __tb_boot_request, val) };
}
