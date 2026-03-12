use core::convert::Infallible;

use embedded_hal::digital::OutputPin;
use embedded_io::ErrorType;

use super::usart::Usart;

/// Active level of the direction pin.
///
/// Controls when the transceiver is in transmit mode:
/// - `ActiveHigh`: pin is driven high to transmit, low to receive.
/// - `ActiveLow`: pin is driven low to transmit, high to receive.
pub enum Polarity {
    ActiveHigh,
    ActiveLow,
}

/// RS-485 / DXL TTL style transport.
///
/// Wraps a `Usart` and toggles a direction pin (DE/TX_EN) around writes
/// to control an external half-duplex transceiver.
pub struct Rs485<P: OutputPin> {
    usart: Usart,
    dir_pin: P,
    polarity: Polarity,
}

impl<P: OutputPin> Rs485<P> {
    pub fn new(usart: Usart, dir_pin: P, polarity: Polarity) -> Self {
        let mut rs485 = Rs485 { usart, dir_pin, polarity };
        rs485.set_rx();
        rs485
    }

    fn set_tx(&mut self) {
        let _ = match self.polarity {
            Polarity::ActiveHigh => self.dir_pin.set_high(),
            Polarity::ActiveLow => self.dir_pin.set_low(),
        };
    }

    fn set_rx(&mut self) {
        let _ = match self.polarity {
            Polarity::ActiveHigh => self.dir_pin.set_low(),
            Polarity::ActiveLow => self.dir_pin.set_high(),
        };
    }
}

impl<P: OutputPin> ErrorType for Rs485<P> {
    type Error = Infallible;
}

impl<P: OutputPin> embedded_io::Read for Rs485<P> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.usart.read(buf)
    }
}

impl<P: OutputPin> embedded_io::Write for Rs485<P> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.set_tx();
        let n = self.usart.write(buf)?;
        self.usart.flush()?;
        self.set_rx();
        Ok(n)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.usart.flush()
    }
}
