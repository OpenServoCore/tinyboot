/// Feed the independent watchdog. No-op if IWDG isn't enabled.
pub fn feed() {
    const IWDG_CTLR: u32 = 0x4000_3000;
    unsafe { core::ptr::write_volatile(IWDG_CTLR as *mut u32, 0xAAAA) };
}
