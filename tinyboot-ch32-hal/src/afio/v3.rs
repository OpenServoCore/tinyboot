#[inline(always)]
pub fn set_usart_remap(n: u8, remap: u8) {
    if remap == 0 {
        return;
    }
    match n {
        1 => ch32_metapac::AFIO
            .pcfr1()
            .modify(|w| w.set_usart1_rm(remap & 1 != 0)),
        2 => ch32_metapac::AFIO
            .pcfr1()
            .modify(|w| w.set_usart2_rm(remap & 1 != 0)),
        3 => ch32_metapac::AFIO
            .pcfr1()
            .modify(|w| w.set_usart3_rm(remap)),
        _ => {}
    }
}
