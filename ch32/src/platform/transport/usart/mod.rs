use core::convert::Infallible;

use embedded_io::ErrorType;

use crate::hal::UsartMapping;
use crate::hal::{afio, rcc, usart};
use crate::platform::boot_meta_store;

mod wire;

use wire::Switch;
pub use wire::{
    FullDrive, FullDuplex, HalfDrive, HalfDuplex, NoTxEn, OpenDrain, PushPull,
    PushPullWhileSending, TxEn, TxEnable, Wire,
};

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

pub struct UsartConfig<W: Wire, E: TxEnable> {
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
    /// [`FullDuplex`] or [`HalfDuplex`], with the TX drive.
    pub wire: W,
    /// [`NoTxEn`] or a [`TxEn`] DE/RE pin.
    pub tx_en: E,
}

pub struct Usart<W: Wire, E: TxEnable> {
    regs: crate::hal::usart::Regs,
    wire: W::Switch,
    tx_en: E::Switch,
}

impl<W: Wire, E: TxEnable> tinyboot_core::traits::Transport for Usart<W, E> {
    #[inline(always)]
    fn wait_bytes(&mut self, n: u8) {
        usart::wait_bytes(self.regs, n);
    }
}

impl<W: Wire, E: TxEnable> Usart<W, E> {
    #[cfg_attr(split_sysflash, inline(never))]
    #[cfg_attr(not(split_sysflash), inline(always))]
    #[cfg_attr(split_sysflash, unsafe(link_section = ".text2"))]
    pub fn new(config: &UsartConfig<W, E>) -> Self {
        let tx_pin = config.mapping.tx_pin();
        let rx_pin = config.mapping.rx_pin();
        let regs = config.mapping.regs();

        let usart_n = config.mapping.peripheral_index();

        // Batch-enable GPIO ports, AFIO, and (if applicable) USART on APB2.
        rcc::enable_apb2(
            (1 << (2 + tx_pin.port_index()))
                | (1 << (2 + rx_pin.port_index()))
                | 1 // AFIO
                | rcc::usart_apb2_bit(usart_n)
                | config.tx_en.apb2_bits(),
        );
        // USART2/3 live on APB1.
        if rcc::usart_apb2_bit(usart_n) == 0 {
            rcc::enable_usart(usart_n);
        }

        afio::set_usart_remap(usart_n, config.mapping.remap_value());

        let tx_en = config.tx_en.init();

        let brr = stored_brr(config.pclk, brr(config.pclk, config.baud.hz()));
        usart::init(regs, brr, W::HALF_DUPLEX);

        // After TE: a TX pin switched to its USART function before then
        // follows an output that idles low.
        let wire = config.wire.init(tx_pin, rx_pin);

        Usart { regs, wire, tx_en }
    }

    #[inline(always)]
    fn set_tx_mode(&self) {
        self.wire.tx();
        self.tx_en.tx();
    }

    #[inline(always)]
    fn set_rx_mode(&self) {
        self.wire.rx();
        self.tx_en.rx();
    }
}

impl<W: Wire, E: TxEnable> ErrorType for Usart<W, E> {
    type Error = Infallible;
}

impl<W: Wire, E: TxEnable> embedded_io::Read for Usart<W, E> {
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

impl<W: Wire, E: TxEnable> embedded_io::Write for Usart<W, E> {
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
