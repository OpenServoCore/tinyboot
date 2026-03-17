pub(crate) fn init(r: ch32_metapac::usart::Usart, pclk: u32, baud: u32, half_duplex: bool) {
    r.ctlr2().modify(|w| w.set_stop(0b00));

    r.ctlr1().modify(|w| {
        w.set_m(false);
        w.set_pce(false);
        w.set_te(true);
        w.set_re(true);
    });

    r.ctlr3().modify(|w| {
        w.set_rtse(false);
        w.set_ctse(false);
        if half_duplex {
            w.set_hdsel(true);
        }
    });

    let brr = (pclk + baud / 2) / baud;
    r.brr().write_value(ch32_metapac::usart::regs::Brr(brr));

    r.ctlr1().modify(|w| w.set_ue(true));
}

pub(crate) fn read_byte(r: ch32_metapac::usart::Usart) -> u8 {
    while !r.statr().read().rxne() {}
    r.datar().read().dr() as u8
}

pub(crate) fn write_byte(r: ch32_metapac::usart::Usart, byte: u8) {
    while !r.statr().read().txe() {}
    r.datar().write(|w| w.set_dr(byte as u16));
}

pub(crate) fn flush(r: ch32_metapac::usart::Usart) {
    while !r.statr().read().tc() {}
}
