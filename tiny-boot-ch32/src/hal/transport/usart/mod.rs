use core::convert::Infallible;

use embedded_io::ErrorType;

#[cfg_attr(usart_common, path = "usart_common.rs")]
mod family;

pub enum Duplex {
    /// Single-wire half-duplex. TX pin is shared for both transmit and receive
    /// via the USART HDSEL bit. No external transceiver needed.
    Half,

    /// Full-duplex with separate TX and RX pins.
    Full,
}

pub enum BaudRate {
    B9600,
    B19200,
    B38400,
    B57600,
    B115200,
    B1000000,
    B2000000,
}

impl BaudRate {
    pub const fn value(&self) -> u32 {
        match self {
            BaudRate::B9600 => 9600,
            BaudRate::B19200 => 19200,
            BaudRate::B38400 => 38400,
            BaudRate::B57600 => 57600,
            BaudRate::B115200 => 115200,
            BaudRate::B1000000 => 1_000_000,
            BaudRate::B2000000 => 2_000_000,
        }
    }
}

/// Configuration for a USART peripheral.
///
/// Caller is responsible for enabling RCC clocks (USART + GPIO) and
/// configuring GPIO pins before constructing `Usart`.
pub struct UsartConfig {
    pub duplex: Duplex,
    pub baud: BaudRate,

    /// Peripheral clock frequency in Hz (e.g. 48_000_000 for CH32V003 default).
    /// Used to calculate the baud rate divisor.
    pub pclk: u32,
}

pub struct Usart {
    regs: ch32_metapac::usart::Usart,
}

impl Usart {
    pub fn new(regs: ch32_metapac::usart::Usart, config: &UsartConfig) -> Self {
        family::init(&regs, config.pclk, config.baud.value(), &config.duplex);
        Usart { regs }
    }
}

impl ErrorType for Usart {
    type Error = Infallible;
}

impl embedded_io::Read for Usart {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        buf[0] = family::read_byte(&self.regs);
        Ok(1)
    }
}

impl embedded_io::Write for Usart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &byte in buf {
            family::write_byte(&self.regs, byte);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        family::flush(&self.regs);
        Ok(())
    }
}
