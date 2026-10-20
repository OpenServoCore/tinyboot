use core::convert::Infallible;

use embedded_io::ErrorType;

use crate::hal::gpio::PinMode;
#[cfg(not(feature = "half-duplex"))]
use crate::hal::gpio::Pull;
use crate::hal::{Pin, UsartMapping};
use crate::hal::{afio, gpio, rcc, usart};
use crate::platform::boot_meta_store;

pub use tinyboot_core::BaudRate;

/// Rounded BRR for `hz`; folds to a constant when both args are.
#[inline(always)]
const fn brr(pclk: u32, hz: u32) -> u32 {
    (pclk + hz / 2) / hz
}

/// BRR for the stored baud index, or `default` when the index is unknown
/// or the rate is out of reach (BRR < 16) from `pclk`. Every arm divides
/// constants, so this compiles to a table lookup with no runtime divide.
#[inline(always)]
fn stored_brr(pclk: u32, default: u32) -> u32 {
    macro_rules! arms {
        ($idx:expr, $($b:ident),*) => {
            match $idx {
                $(i if i == BaudRate::$b as u8 => brr(pclk, BaudRate::$b.hz()),)*
                _ => default,
            }
        };
    }
    let v = arms!(
        boot_meta_store::stored_baud(),
        B9600,
        B19200,
        B38400,
        B57600,
        B115200,
        B230400,
        B460800,
        B500000,
        B921600,
        B1000000,
        B1500000,
        B2000000,
        B2500000,
        B3000000
    );
    if v < 16 { default } else { v }
}

/// RS-485 DE/RE pin configuration.
#[derive(Copy, Clone)]
pub struct TxEnConfig {
    pub pin: Pin,
    /// Level driven to enable TX; the inverse enables RX.
    pub tx_level: gpio::Level,
}

pub struct UsartConfig {
    /// Build default. A baud stored in boot metadata (`Reset` with the
    /// `BAUD` flag, or the app's `set_baud`) overrides it when `pclk`
    /// can reach it.
    pub baud: BaudRate,
    /// USART peripheral clock (Hz). Used for BRR = pclk / baud.
    ///
    /// PCLK2 at reset (OpenWCH SDK defaults):
    ///   V003: HSI 24MHz / HPRE 3 = 8_000_000
    ///   V1xx/V2xx/V3xx: HSI 8MHz / HPRE 1 = 8_000_000
    ///   X0xx: HSI 48MHz / HPRE 1 = 48_000_000
    pub pclk: u32,
    pub mapping: UsartMapping,
    /// RX pin bias (full duplex only).
    #[cfg(not(feature = "half-duplex"))]
    pub rx_pull: Pull,
    /// Optional RS-485 DE/RE pin.
    pub tx_en: Option<TxEnConfig>,
}

pub struct Usart {
    regs: crate::hal::usart::Regs,
    tx_en: Option<TxEnConfig>,
    /// The single wire: push-pull while driving, open-drain parked.
    #[cfg(feature = "half-duplex")]
    tx_pin: Pin,
}

impl tinyboot_core::traits::Transport for Usart {}

impl Usart {
    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(not(split_sysflash), inline(always))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
    pub fn new(config: &UsartConfig) -> Self {
        let tx_pin = config.mapping.tx_pin();
        let rx_pin = config.mapping.rx_pin();
        let regs = config.mapping.regs();

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

        // Park the single wire open-drain: RX hears through the pin, the
        // bus pull-up holds mark; writes flip to push-pull.
        #[cfg(feature = "half-duplex")]
        gpio::configure(tx_pin, PinMode::AF_OPEN_DRAIN);
        #[cfg(not(feature = "half-duplex"))]
        {
            gpio::configure(tx_pin, PinMode::AF_PUSH_PULL);
            gpio::configure(rx_pin, PinMode::input_pull(config.rx_pull));
        }

        // Start DE/RE in RX mode.
        if let Some(ref tx_en) = config.tx_en {
            rcc::enable_gpio(tx_en.pin.port_index());
            gpio::configure(tx_en.pin, PinMode::OUTPUT_PUSH_PULL);
            gpio::set_level(tx_en.pin, invert(tx_en.tx_level));
        }

        let brr = stored_brr(config.pclk, brr(config.pclk, config.baud.hz()));
        usart::init(regs, brr, cfg!(feature = "half-duplex"));

        Usart {
            regs,
            tx_en: config.tx_en,
            #[cfg(feature = "half-duplex")]
            tx_pin,
        }
    }

    #[inline(always)]
    fn set_tx_mode(&self) {
        #[cfg(feature = "half-duplex")]
        gpio::configure(self.tx_pin, PinMode::AF_PUSH_PULL);
        if let Some(ref tx_en) = self.tx_en {
            gpio::set_level(tx_en.pin, tx_en.tx_level);
        }
    }

    #[inline(always)]
    fn set_rx_mode(&self) {
        #[cfg(feature = "half-duplex")]
        gpio::configure(self.tx_pin, PinMode::AF_OPEN_DRAIN);
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
    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        buf[0] = usart::read_byte(self.regs);
        Ok(1)
    }

    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
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
    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.set_tx_mode();
        let regs = self.regs;
        for &byte in buf {
            usart::write_byte(regs, byte);
        }
        Ok(())
    }

    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
    fn flush(&mut self) -> Result<(), Self::Error> {
        usart::flush(self.regs);
        self.set_rx_mode();
        Ok(())
    }
}
