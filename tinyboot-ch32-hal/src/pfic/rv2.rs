/// Jump to an absolute address. Does not return.
#[allow(asm_sub_register)]
pub fn jump(addr: u32) -> ! {
    unsafe { core::arch::asm!("jr {0}", in(reg) addr, options(noreturn)) };
}

pub fn system_reset() -> ! {
    ch32_metapac::PFIC.cfgr().write(|w| {
        w.set_keycode(0xBEEF);
        w.set_resetsys(true);
    });
    loop {
        core::hint::spin_loop();
    }
}
