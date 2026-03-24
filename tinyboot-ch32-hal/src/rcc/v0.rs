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
