use core::convert::Infallible;

use embedded_io::ErrorType;

pub(crate) struct Ch32Transport;

impl Ch32Transport {
    pub fn new() -> Self {
        Ch32Transport
    }
}

impl ErrorType for Ch32Transport {
    type Error = Infallible;
}

impl embedded_io::Read for Ch32Transport {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}

impl embedded_io::Write for Ch32Transport {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}
