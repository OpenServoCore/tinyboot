pub fn enable_gpio(port_index: usize) {
    ch32_metapac::RCC.apb2pcenr().modify(|w| match port_index {
        0 => w.set_iopaen(true),
        2 => w.set_iopcen(true),
        3 => w.set_iopden(true),
        _ => {}
    });
}

pub(crate) fn enable_afio() {
    ch32_metapac::RCC.apb2pcenr().modify(|w| w.set_afioen(true));
}

pub(crate) fn enable_usart1() {
    ch32_metapac::RCC
        .apb2pcenr()
        .modify(|w| w.set_usart1en(true));
}
