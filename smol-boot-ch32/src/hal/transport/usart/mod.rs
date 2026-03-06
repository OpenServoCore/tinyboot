use core::convert::Infallible;

use embedded_io::ErrorType;

#[cfg_attr(usart_common, path = "usart_common.rs")]
mod family;

pub struct Usart {
    regs: ch32_metapac::usart::Usart,
}

impl Usart {
    pub fn new(regs: ch32_metapac::usart::Usart) -> Self {
        Usart { regs }
    }
}

impl ErrorType for Usart {
    type Error = Infallible;
}

impl embedded_io::Read for Usart {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}

impl embedded_io::Write for Usart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}
