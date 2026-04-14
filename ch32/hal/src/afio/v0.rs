#[inline(always)]
pub fn set_usart_remap(n: u8, remap: u8) {
    if remap == 0 {
        return;
    }
    if n == 1 {
        ch32_metapac::AFIO.pcfr1().write(|w| {
            w.set_usart1_rm(remap & 1 != 0);
            w.set_usart1_rm1(remap & 2 != 0);
        });
    }
}
