use core::convert::Infallible;

use embedded_io::ErrorType;

pub(crate) struct Ch32Uart;

impl Ch32Uart {
    pub fn new() -> Self {
        Ch32Uart
    }
}

impl ErrorType for Ch32Uart {
    type Error = Infallible;
}

impl embedded_io::Read for Ch32Uart {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}

impl embedded_io::Write for Ch32Uart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}
