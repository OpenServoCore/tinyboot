#[inline(always)]
pub fn set_usart1_remap(remap: u8) {
    ch32_metapac::AFIO.pcfr1().write(|w| {
        w.set_usart1_rm(remap & 1 != 0);
        w.set_usart1_rm1(remap & 2 != 0);
    });
}
