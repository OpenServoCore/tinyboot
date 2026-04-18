pub fn enable_gpio(port_index: usize) {
    // IOPxEN starts at bit 2: IOPA=2, IOPB=3, IOPC=4, IOPD=5.
    ch32_metapac::RCC
        .apb2pcenr()
        .modify(|w| w.0 |= 1 << (2 + port_index));
}

pub fn enable_afio() {
    ch32_metapac::RCC.apb2pcenr().modify(|w| w.set_afioen(true));
}

const USART1EN: u32 = 1 << 14;

/// APB2 bit for USART `n`, 0 if not on APB2.
pub const fn usart_apb2_bit(n: u8) -> u32 {
    match n {
        1 => USART1EN,
        _ => 0,
    }
}

pub fn enable_usart(n: u8) {
    let bit = usart_apb2_bit(n);
    if bit != 0 {
        ch32_metapac::RCC.apb2pcenr().modify(|w| w.0 |= bit);
    } else {
        let rcc = ch32_metapac::RCC;
        match n {
            2 => rcc.apb1pcenr().modify(|w| w.set_usart2en(true)),
            3 => rcc.apb1pcenr().modify(|w| w.set_usart3en(true)),
            _ => {}
        }
    }
}

/// Set APB2 enables in one write. Safe only during init.
#[inline(always)]
pub fn enable_apb2(bits: u32) {
    ch32_metapac::RCC.apb2pcenr().write(|w| w.0 = bits);
}

/// Pulse-reset and disable all APB2 peripherals.
#[inline(always)]
pub fn reset_apb2() {
    let rcc = ch32_metapac::RCC;
    let enabled = rcc.apb2pcenr().read().0;
    rcc.apb2prstr().write(|w| w.0 = enabled);
    rcc.apb2prstr().write(|w| w.0 = 0);
    rcc.apb2pcenr().write(|w| w.0 = 0);
}
