pub use ch32_metapac::usart::Usart as Regs;

#[inline(always)]
pub fn init(r: Regs, pclk: u32, baud: u32, half_duplex: bool) {
    // 8N1: write zeroes all bits (M=0, PCE=0, STOP=0b00), then set TE+RE
    r.ctlr1().write(|w| {
        w.set_te(true);
        w.set_re(true);
    });

    // RTSE=0, CTSE=0 are default; only touch CTLR3 for half-duplex
    if half_duplex {
        r.ctlr3().write(|w| w.set_hdsel(true));
    }

    let brr = (pclk + baud / 2) / baud;
    r.brr().write_value(ch32_metapac::usart::regs::Brr(brr));

    r.ctlr1().modify(|w| w.set_ue(true));
}

#[inline(always)]
pub fn read_byte(r: ch32_metapac::usart::Usart) -> u8 {
    while !r.statr().read().rxne() {}
    r.datar().read().dr() as u8
}

#[inline(always)]
pub fn write_byte(r: ch32_metapac::usart::Usart, byte: u8) {
    while !r.statr().read().txe() {}
    r.datar().write(|w| w.set_dr(byte as u16));
}

#[inline(always)]
pub fn flush(r: ch32_metapac::usart::Usart) {
    while !r.statr().read().tc() {}
}
