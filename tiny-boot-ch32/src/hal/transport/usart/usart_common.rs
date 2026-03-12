use ch32_metapac::usart::Usart;

use super::Duplex;

/// Configure USART registers: baud rate, 8N1, duplex mode, then enable.
///
/// Follows the WCH SDK initialization order:
/// 1. CTLR2 (stop bits)
/// 2. CTLR1 (word length, parity, TE/RE)
/// 3. CTLR3 (flow control, half-duplex)
/// 4. BRR (baud rate)
/// 5. Enable UE via modify
///
/// Caller must enable RCC clocks (USART + GPIO) and configure GPIO pins
/// before calling this.
pub(super) fn init(regs: &Usart, pclk: u32, baud: u32, duplex: &Duplex) {
    // 1. Stop bits: 1 stop bit (STOP=0b00)
    regs.ctlr2().modify(|w| w.set_stop(0b00));

    // 2. Word length 8-bit (M=0), no parity (PCE=0), enable TX and RX
    regs.ctlr1().modify(|w| {
        w.set_m(false);
        w.set_pce(false);
        w.set_te(true);
        w.set_re(true);
    });

    // 3. Half-duplex mode if requested, no hardware flow control
    regs.ctlr3().modify(|w| {
        w.set_rtse(false);
        w.set_ctse(false);
        if matches!(duplex, Duplex::Half) {
            w.set_hdsel(true);
        }
    });

    // 4. Baud rate: BRR = PCLK / baud
    // The 16x oversampling factor is encoded in the register layout itself:
    // [15:4] = mantissa, [3:0] = fraction of USARTDIV.
    // PCLK / baud gives USARTDIV * 16, which is the raw BRR value.
    let brr = (pclk + baud / 2) / baud;
    regs.brr().write_value(ch32_metapac::usart::regs::Brr(brr));

    // 5. Enable USART
    regs.ctlr1().modify(|w| w.set_ue(true));
}

/// Block until a byte is received, then return it.
pub(super) fn read_byte(regs: &Usart) -> u8 {
    while !regs.statr().read().rxne() {}
    regs.datar().read().dr() as u8
}

/// Block until the TX data register is empty, then write a byte.
pub(super) fn write_byte(regs: &Usart, byte: u8) {
    while !regs.statr().read().txe() {}
    regs.datar().write(|w| w.set_dr(byte as u16));
}

/// Block until the last transmission is fully complete (shift register empty).
pub(super) fn flush(regs: &Usart) {
    while !regs.statr().read().tc() {}
}
