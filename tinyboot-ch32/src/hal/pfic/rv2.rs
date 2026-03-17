pub(crate) fn system_reset() -> ! {
    ch32_metapac::PFIC.cfgr().write(|w| {
        w.set_keycode(0xBEEF);
        w.set_resetsys(true);
    });
    loop {
        core::hint::spin_loop();
    }
}
