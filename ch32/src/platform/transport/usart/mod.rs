use core::convert::Infallible;

use embedded_io::ErrorType;

use crate::hal::gpio::{PinMode, Pull};
use crate::hal::{Pin, UsartMapping};
use crate::hal::{afio, gpio, rcc, usart};

pub enum Duplex {
    /// Half-duplex (single wire, RS-485).
    Half,
    /// Full-duplex (separate TX/RX).
    Full,
}

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum BaudRate {
    B9600 = 9600,
    B19200 = 19200,
    B38400 = 38400,
    B57600 = 57600,
    B115200 = 115200,
}

/// RS-485 DE/RE pin configuration.
#[derive(Copy, Clone)]
pub struct TxEnConfig {
    pub pin: Pin,
    /// Level driven to enable TX; the inverse enables RX.
    pub tx_level: gpio::Level,
}

pub struct UsartConfig {
    pub duplex: Duplex,
    pub baud: BaudRate,
    /// USART peripheral clock (Hz). Used for BRR = pclk / baud.
    ///
    /// PCLK2 at reset (OpenWCH SDK defaults):
    ///   V003: HSI 24MHz / HPRE 3 = 8_000_000
    ///   V1xx/V2xx/V3xx: HSI 8MHz / HPRE 1 = 8_000_000
    ///   X0xx: HSI 48MHz / HPRE 1 = 48_000_000
    pub pclk: u32,
    pub mapping: UsartMapping,
    pub rx_pull: Pull,
    /// Optional RS-485 DE/RE pin.
    pub tx_en: Option<TxEnConfig>,
}

pub struct Usart {
    regs: crate::hal::usart::Regs,
    tx_en: Option<TxEnConfig>,
}

impl tinyboot_core::traits::Transport for Usart {}

impl Usart {
    #[inline(always)]
    pub fn new(config: &UsartConfig) -> Self {
        let tx_pin = config.mapping.tx_pin();
        let rx_pin = config.mapping.rx_pin();
        let regs = config.mapping.regs();
        let half_duplex = matches!(config.duplex, Duplex::Half);

        let usart_n = config.mapping.peripheral_index();

        // Batch-enable GPIO ports, AFIO, and (if applicable) USART on APB2.
        rcc::enable_apb2(
            (1 << (2 + tx_pin.port_index()))
                | (1 << (2 + rx_pin.port_index()))
                | 1 // AFIO
                | rcc::usart_apb2_bit(usart_n),
        );
        // USART2/3 live on APB1.
        if rcc::usart_apb2_bit(usart_n) == 0 {
            rcc::enable_usart(usart_n);
        }

        afio::set_usart_remap(usart_n, config.mapping.remap_value());

        gpio::configure(tx_pin, PinMode::AF_PUSH_PULL);
        if !half_duplex {
            gpio::configure(rx_pin, PinMode::input_pull(config.rx_pull));
        }

        // Start DE/RE in RX mode.
        if let Some(ref tx_en) = config.tx_en {
            rcc::enable_gpio(tx_en.pin.port_index());
            gpio::configure(tx_en.pin, PinMode::OUTPUT_PUSH_PULL);
            gpio::set_level(tx_en.pin, invert(tx_en.tx_level));
        }

        usart::init(regs, config.pclk, config.baud as u32, half_duplex);

        Usart {
            regs,
            tx_en: config.tx_en,
        }
    }

    #[inline(always)]
    fn set_tx_mode(&self) {
        if let Some(ref tx_en) = self.tx_en {
            gpio::set_level(tx_en.pin, tx_en.tx_level);
        }
    }

    #[inline(always)]
    fn set_rx_mode(&self) {
        if let Some(ref tx_en) = self.tx_en {
            gpio::set_level(tx_en.pin, invert(tx_en.tx_level));
        }
    }
}

#[inline(always)]
fn invert(level: gpio::Level) -> gpio::Level {
    match level {
        gpio::Level::High => gpio::Level::Low,
        gpio::Level::Low => gpio::Level::High,
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
        buf[0] = usart::read_byte(self.regs);
        Ok(1)
    }

    fn read_exact(
        &mut self,
        buf: &mut [u8],
    ) -> Result<(), embedded_io::ReadExactError<Self::Error>> {
        let regs = self.regs;
        for byte in buf {
            *byte = usart::read_byte(regs);
        }
        Ok(())
    }
}

impl embedded_io::Write for Usart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.set_tx_mode();
        let regs = self.regs;
        for &byte in buf {
            usart::write_byte(regs, byte);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        usart::flush(self.regs);
        self.set_rx_mode();
        Ok(())
    }
}
