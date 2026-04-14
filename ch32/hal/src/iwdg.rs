/// Feed the independent watchdog timer.
///
/// Writes the reload key (0xAAAA) to IWDG_CTLR. Safe to call even if
/// the watchdog is not enabled — the write is simply ignored.
pub fn feed() {
    const IWDG_CTLR: u32 = 0x4000_3000;
    unsafe { core::ptr::write_volatile(IWDG_CTLR as *mut u32, 0xAAAA) };
}
