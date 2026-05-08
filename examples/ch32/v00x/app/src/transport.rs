//! embedded_io adapters for ch32-hal blocking UART with optional RS-485 DE/RE.
//!
//! RX bypasses ch32-hal's `blocking_read` and polls `STATR.RXNE` / `DATAR`
//! directly — at 48 MHz with a 3 Mbps line, each byte arrives every ~160
//! cycles and the per-byte overhead of `check_rx_flags` + the `embedded_io`
//! adapter chain is enough to overrun.

use ch32_hal::gpio::{Level, Output};
use ch32_hal::mode::Blocking;
use ch32_hal::pac;
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
        let regs = pac::USART1;
        for slot in buf.iter_mut() {
            while !regs.statr().read().rxne() {}
            *slot = regs.datar().read().dr() as u8;
        }
        Ok(buf.len())
    }
}

pub struct TxEn<'d> {
    pub pin: Output<'d>,
    /// Level driven to enable TX; the inverse enables RX.
    pub tx_level: Level,
}

impl TxEn<'_> {
    #[inline(always)]
    fn set_tx(&mut self) {
        self.pin.set_level(self.tx_level);
    }

    #[inline(always)]
    fn set_rx(&mut self) {
        self.pin.set_level(invert(self.tx_level));
    }
}

#[inline(always)]
pub fn invert(level: Level) -> Level {
    match level {
        Level::High => Level::Low,
        Level::Low => Level::High,
    }
}

pub struct Tx<'d, T: Instance> {
    pub uart: UartTx<'d, T, Blocking>,
    pub tx_en: Option<TxEn<'d>>,
}

impl<T: Instance> ErrorType for Tx<'_, T> {
    type Error = Infallible;
}

impl<T: Instance> Write for Tx<'_, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if let Some(tx_en) = self.tx_en.as_mut() {
            tx_en.set_tx();
        }
        let _ = self.uart.blocking_write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        let _ = self.uart.blocking_flush();
        if let Some(tx_en) = self.tx_en.as_mut() {
            tx_en.set_rx();
        }
        Ok(())
    }
}
