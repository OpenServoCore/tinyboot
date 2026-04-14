pub fn enable_gpio(port_index: usize) {
    // IOPxEN bits are consecutive starting at bit 2: IOPA=2, IOPB=3, IOPC=4, IOPD=5.
    ch32_metapac::RCC
        .apb2pcenr()
        .modify(|w| w.0 |= 1 << (2 + port_index));
}

pub fn enable_afio() {
    ch32_metapac::RCC.apb2pcenr().modify(|w| w.set_afioen(true));
}

pub fn enable_usart1() {
    ch32_metapac::RCC
        .apb2pcenr()
        .modify(|w| w.set_usart1en(true));
}

/// Batch-enable APB2 peripherals in a single write.
/// Only safe at init before other peripherals are enabled.
#[inline(always)]
pub fn enable_apb2(bits: u32) {
    ch32_metapac::RCC.apb2pcenr().write(|w| w.0 = bits);
}

/// Pulse-reset all APB2 peripherals (USART1, GPIO, AFIO, etc.)
/// then disable their clocks — restores power-on default state.
#[inline(always)]
pub fn reset_apb2() {
    let rcc = ch32_metapac::RCC;
    let enabled = rcc.apb2pcenr().read().0;
    rcc.apb2prstr().write(|w| w.0 = enabled);
    rcc.apb2prstr().write(|w| w.0 = 0);
    rcc.apb2pcenr().write(|w| w.0 = 0);
}
