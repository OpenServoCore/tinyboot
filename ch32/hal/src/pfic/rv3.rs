/// Jump to an absolute address. Does not return.
pub fn jump(addr: u32) -> ! {
    let f: unsafe extern "C" fn() -> ! = unsafe { core::mem::transmute(addr as usize) };
    unsafe { f() }
}

pub fn system_reset() -> ! {
    use ch32_metapac::pfic::vals::{Keycode, Sysreset};

    // Clear reset status flags (RMVF) — required for boot mode transition
    ch32_metapac::RCC.rstsckr().write(|w| w.0 = 1 << 24);
    ch32_metapac::PFIC.cfgr().write(|w| {
        w.set_keycode(Keycode(0xBEEF));
        w.set_sysreset(Sysreset::RESET);
    });
    loop {
        core::hint::spin_loop();
    }
}
