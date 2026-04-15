use core::convert::Infallible;

use embedded_io::ErrorType;

use tinyboot_ch32_hal::gpio::{PinMode, Pull};
use tinyboot_ch32_hal::{Pin, UsartMapping};
use tinyboot_ch32_hal::{afio, gpio, rcc, usart};
/// USART duplex mode.
pub enum Duplex {
    /// Half-duplex (single wire, RS-485).
    Half,
    /// Full-duplex (separate TX/RX).
    Full,
}

/// Supported baud rates.
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum BaudRate {
    /// 9600 baud.
    B9600 = 9600,
    /// 19200 baud.
    B19200 = 19200,
    /// 38400 baud.
    B38400 = 38400,
    /// 57600 baud.
    B57600 = 57600,
    /// 115200 baud.
    B115200 = 115200,
}

/// TX-enable (DE/RE) pin configuration for RS-485 transceivers.
#[derive(Copy, Clone)]
pub struct TxEnConfig {
    /// GPIO pin connected to DE/RE.
    pub pin: Pin,
    /// `true` if the transceiver enables TX on logic high.
    pub active_high: bool,
}

/// USART peripheral configuration.
pub struct UsartConfig {
    /// Half- or full-duplex mode.
    pub duplex: Duplex,
    /// Baud rate.
    pub baud: BaudRate,
    /// Peripheral clock frequency (Hz) feeding this USART.
    /// Used to compute the baud rate divisor (BRR = pclk / baud).
    ///
    /// Default PCLK2 at reset from OpenWCH SDK headers:
    ///   CH32V003: HSI=24MHz / HPRE=3 = 8_000_000
    ///   CH32V1xx/V2xx/V3xx: HSI=8MHz / HPRE=1 = 8_000_000
    ///   CH32X0xx: HSI=48MHz / HPRE=1 = 48_000_000
    pub pclk: u32,
    /// USART pin mapping (selects TX/RX pins and remap).
    pub mapping: UsartMapping,
    /// RX pin pull configuration.
    pub rx_pull: Pull,
    /// Optional TX-enable pin for RS-485 DE/RE control.
    pub tx_en: Option<TxEnConfig>,
}

/// USART transport with optional RS-485 TX-enable control.
pub struct Usart {
    regs: tinyboot_ch32_hal::usart::Regs,
    tx_en: Option<TxEnConfig>,
}

impl tinyboot::traits::boot::Transport for Usart {}

impl Usart {
    /// Initialize the USART peripheral with the given configuration.
    #[inline(always)]
    pub fn new(config: &UsartConfig) -> Self {
        let tx_pin = config.mapping.tx_pin();
        let rx_pin = config.mapping.rx_pin();
        let regs = config.mapping.regs();
        let half_duplex = matches!(config.duplex, Duplex::Half);

        let usart_n = config.mapping.peripheral_index();

        // Batch-enable GPIO port(s), AFIO, and USART clocks
        rcc::enable_apb2(
            (1 << (2 + tx_pin.port_index()))
                | (1 << (2 + rx_pin.port_index()))
                | 1 // AFIO
                | rcc::usart_apb2_bit(usart_n),
        );
        // Enable USART on APB1 if not on APB2 (e.g. USART2/3)
        if rcc::usart_apb2_bit(usart_n) == 0 {
            rcc::enable_usart(usart_n);
        }

        // Set pin remap
        afio::set_usart_remap(usart_n, config.mapping.remap_value());

        // Configure pins
        gpio::configure(tx_pin, PinMode::AF_PUSH_PULL);
        if !half_duplex {
            gpio::configure(rx_pin, PinMode::input_pull(config.rx_pull));
        }

        // Configure TX_EN pin if present (start in RX mode)
        if let Some(ref tx_en) = config.tx_en {
            rcc::enable_gpio(tx_en.pin.port_index());
            gpio::configure(tx_en.pin, PinMode::OUTPUT_PUSH_PULL);
            let rx_level = if tx_en.active_high {
                gpio::Level::Low
            } else {
                gpio::Level::High
            };
            gpio::set_level(tx_en.pin, rx_level);
        }

        // Initialize USART
        usart::init(regs, config.pclk, config.baud as u32, half_duplex);

        Usart {
            regs,
            tx_en: config.tx_en,
        }
    }

    #[inline(always)]
    fn set_tx_mode(&self) {
        if let Some(ref tx_en) = self.tx_en {
            let level = if tx_en.active_high {
                gpio::Level::High
            } else {
                gpio::Level::Low
            };
            gpio::set_level(tx_en.pin, level);
        }
    }

    #[inline(always)]
    fn set_rx_mode(&self) {
        if let Some(ref tx_en) = self.tx_en {
            let level = if tx_en.active_high {
                gpio::Level::Low
            } else {
                gpio::Level::High
            };
            gpio::set_level(tx_en.pin, level);
        }
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
