use core::convert::Infallible;

use embedded_io::ErrorType;

use tinyboot_ch32_hal::gpio::{PinMode, Pull};
use tinyboot_ch32_hal::{Pin, UsartMapping};
use tinyboot_ch32_hal::{afio, gpio, rcc, usart};
use tinyboot_protocol::frame::payload_size;

pub enum Duplex {
    Half,
    Full,
}

pub enum BaudRate {
    B9600,
    B19200,
    B38400,
    B57600,
    B115200,
}

impl BaudRate {
    pub fn value(&self) -> u32 {
        match self {
            BaudRate::B9600 => 9600,
            BaudRate::B19200 => 19200,
            BaudRate::B38400 => 38400,
            BaudRate::B57600 => 57600,
            BaudRate::B115200 => 115200,
        }
    }
}

#[derive(Copy, Clone)]
pub struct TxEnConfig {
    pub pin: Pin,
    pub active_high: bool,
}

pub struct UsartConfig {
    pub duplex: Duplex,
    pub baud: BaudRate,
    /// Peripheral clock frequency (Hz) feeding this USART.
    /// Used to compute the baud rate divisor (BRR = pclk / baud).
    ///
    /// Default PCLK2 at reset from OpenWCH SDK headers:
    ///   CH32V003: HSI=24MHz / HPRE=3 = 8_000_000
    ///   CH32V1xx/V2xx/V3xx: HSI=8MHz / HPRE=1 = 8_000_000
    ///   CH32X0xx: HSI=48MHz / HPRE=1 = 48_000_000
    pub pclk: u32,
    pub mapping: UsartMapping,
    pub rx_pull: Pull,
    pub tx_en: Option<TxEnConfig>,
}

pub struct Usart {
    regs: ch32_metapac::usart::Usart,
    tx_en: Option<TxEnConfig>,
}

impl Usart {
    pub const FRAME_SIZE: usize = 64;
    pub const PAYLOAD_SIZE: usize = payload_size(Self::FRAME_SIZE);
}

impl tinyboot::traits::Transport<{ Usart::PAYLOAD_SIZE }> for Usart {}

impl Usart {
    pub fn new(config: &UsartConfig) -> Self {
        let tx_pin = config.mapping.tx_pin();
        let rx_pin = config.mapping.rx_pin();
        let remap = config.mapping.remap_value();
        let regs = config.mapping.regs();
        let half_duplex = matches!(config.duplex, Duplex::Half);

        // Enable clocks
        rcc::enable_gpio(tx_pin.port_index());
        if rx_pin.port_index() != tx_pin.port_index() {
            rcc::enable_gpio(rx_pin.port_index());
        }
        rcc::enable_afio();
        rcc::enable_usart1();

        // Set pin remap if non-default
        if remap != 0 {
            afio::set_usart1_remap(remap);
        }

        // Configure pins
        gpio::configure(tx_pin, PinMode::AfPushPull);
        if !half_duplex {
            gpio::configure(rx_pin, PinMode::InputPull(config.rx_pull));
        }

        // Configure TX_EN pin if present (start in RX mode)
        if let Some(ref tx_en) = config.tx_en {
            rcc::enable_gpio(tx_en.pin.port_index());
            gpio::configure(tx_en.pin, PinMode::OutputPushPull);
            if tx_en.active_high {
                gpio::set_low(tx_en.pin);
            } else {
                gpio::set_high(tx_en.pin);
            }
        }

        // Initialize USART
        usart::init(regs, config.pclk, config.baud.value(), half_duplex);

        Usart {
            regs,
            tx_en: config.tx_en,
        }
    }

    fn set_tx_mode(&self) {
        if let Some(ref tx_en) = self.tx_en {
            if tx_en.active_high {
                gpio::set_high(tx_en.pin);
            } else {
                gpio::set_low(tx_en.pin);
            }
        }
    }

    fn set_rx_mode(&self) {
        if let Some(ref tx_en) = self.tx_en {
            if tx_en.active_high {
                gpio::set_low(tx_en.pin);
            } else {
                gpio::set_high(tx_en.pin);
            }
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
}

impl embedded_io::Write for Usart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.set_tx_mode();
        for &byte in buf {
            usart::write_byte(self.regs, byte);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        usart::flush(self.regs);
        self.set_rx_mode();
        Ok(())
    }
}
