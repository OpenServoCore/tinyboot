pub use ch32_metapac::usart::Usart as Regs;

#[inline(always)]
pub fn init(r: Regs, brr: u32, half_duplex: bool) {
    // HDSEL before TE — the reverse order latches the TX output LOW until
    // the first own transmission, clamping the wire from init.
    if half_duplex {
        r.ctlr3().write(|w| w.set_hdsel(true));
    }

    // CTLR1 default (M=0, PCE=0, STOP=00) gives 8N1; just set TE+RE.
    r.ctlr1().write(|w| {
        w.set_te(true);
        w.set_re(true);
    });

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

/// Spin at least `n` byte-times: BRR is PCLK cycles per bit, each spin
/// takes at least one cycle, and a byte is counted as 16 bits.
#[inline(always)]
pub fn wait_bytes(r: ch32_metapac::usart::Usart, n: u8) {
    let bit = r.brr().read().0;
    // Nested, not one n * 16 * bit count: no multiply on V003.
    let mut bits = (n as u32) << 4;
    while bits != 0 {
        // Bottom-tested: an enabled USART's BRR is at least 16.
        let mut c = bit;
        loop {
            // SAFETY: empty asm, only keeps the loop.
            unsafe { core::arch::asm!("") };
            c -= 1;
            if c == 0 {
                break;
            }
        }
        bits -= 1;
    }
}

#[inline(always)]
pub fn flush(r: ch32_metapac::usart::Usart) {
    while !r.statr().read().tc() {}
}
