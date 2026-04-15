//! embedded_io adapters for ch32-hal blocking UART.

use ch32_hal::mode::Blocking;
use ch32_hal::usart::{Instance, UartRx, UartTx};
use core::convert::Infallible;
use embedded_io::{ErrorType, Read, Write};

pub struct Rx<'d, T: Instance>(pub UartRx<'d, T, Blocking>);

impl<T: Instance> ErrorType for Rx<'_, T> {
    type Error = Infallible;
}

impl<T: Instance> Read for Rx<'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        let _ = self.0.blocking_read(&mut buf[..1]);
        Ok(1)
    }
}

pub struct Tx<'d, T: Instance>(pub UartTx<'d, T, Blocking>);

impl<T: Instance> ErrorType for Tx<'_, T> {
    type Error = Infallible;
}

impl<T: Instance> Write for Tx<'_, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let _ = self.0.blocking_write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        let _ = self.0.blocking_flush();
        Ok(())
    }
}
