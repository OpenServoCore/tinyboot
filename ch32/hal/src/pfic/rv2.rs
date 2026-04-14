/// Jump to an absolute address. Does not return.
pub fn jump(addr: u32) -> ! {
    let f: unsafe extern "C" fn() -> ! = unsafe { core::mem::transmute(addr as usize) };
    unsafe { f() }
}

pub fn system_reset() -> ! {
    // Clear reset status flags (RMVF) — required for boot mode transition
    ch32_metapac::RCC.rstsckr().write(|w| w.0 = 1 << 24);
    ch32_metapac::PFIC.cfgr().write(|w| {
        w.set_keycode(0xBEEF);
        w.set_resetsys(true);
    });
    loop {
        core::hint::spin_loop();
    }
}
