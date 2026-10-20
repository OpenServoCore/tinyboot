//! Wire arrangement and DE/RE pin, chosen by type so a bootloader builds
//! only the per-frame pin switching its board needs. A choice with nothing
//! to switch compiles to no code and no state; a choice that would make an
//! idle node drive a shared wire does not compile.

use crate::hal::gpio::{self, Level, PinMode, Pull};
use crate::hal::{Pin, rcc};

mod sealed {
    pub trait Sealed {}
}
use sealed::Sealed;

/// Per-frame pin switch: [`tx`](Switch::tx) before the first byte of a
/// write, [`rx`](Switch::rx) once the last byte has left the shifter.
pub trait Switch {
    fn tx(&self);
    fn rx(&self);
}

/// Nothing to switch.
impl Switch for () {
    #[inline(always)]
    fn tx(&self) {}
    #[inline(always)]
    fn rx(&self) {}
}

/// Clears a CNF0 bit to drive push-pull, sets it to park open-drain: the
/// two AF modes differ in that bit only.
pub struct DriveSwitch {
    cfgr: *mut u32,
    cnf0: u32,
}

impl Switch for DriveSwitch {
    #[inline(always)]
    fn tx(&self) {
        // SAFETY: GPIO config register address from `gpio::cnf0`.
        unsafe {
            self.cfgr
                .write_volatile(self.cfgr.read_volatile() & !self.cnf0)
        }
    }
    #[inline(always)]
    fn rx(&self) {
        // SAFETY: as in `tx`.
        unsafe {
            self.cfgr
                .write_volatile(self.cfgr.read_volatile() | self.cnf0)
        }
    }
}

/// One BSHR store per direction.
pub struct LevelSwitch {
    bshr: *mut u32,
    tx_word: u32,
    rx_word: u32,
}

impl Switch for LevelSwitch {
    #[inline(always)]
    fn tx(&self) {
        // SAFETY: BSHR address from `gpio::level_word`.
        unsafe { self.bshr.write_volatile(self.tx_word) }
    }
    #[inline(always)]
    fn rx(&self) {
        // SAFETY: as in `tx`.
        unsafe { self.bshr.write_volatile(self.rx_word) }
    }
}

/// TX pin push-pull at all times.
pub struct PushPull;
/// TX pin open-drain at all times: the wire's pull-up makes every rising
/// edge, so its RC sets the highest usable baud.
pub struct OpenDrain;
/// TX pin parked open-drain, push-pull only while sending: sharp edges
/// without driving the wire between frames.
pub struct PushPullWhileSending;

/// TX drive for a [`FullDuplex`] wire.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a full-duplex TX drive",
    label = "full duplex takes `PushPull` or `OpenDrain`"
)]
pub trait FullDrive: Sealed {
    #[doc(hidden)]
    const MODE: PinMode;
}

impl Sealed for PushPull {}
impl Sealed for OpenDrain {}
impl Sealed for PushPullWhileSending {}

impl FullDrive for PushPull {
    const MODE: PinMode = PinMode::AF_PUSH_PULL;
}
impl FullDrive for OpenDrain {
    const MODE: PinMode = PinMode::AF_OPEN_DRAIN;
}

/// TX drive for a [`HalfDuplex`] wire. Every choice leaves the pin
/// open-drain while listening; there is no always-push-pull.
#[diagnostic::on_unimplemented(
    message = "`{Self}` would drive the single wire while listening",
    label = "half duplex takes `PushPullWhileSending` or `OpenDrain`"
)]
pub trait HalfDrive: Sealed {
    #[doc(hidden)]
    type Switch: Switch;
    #[doc(hidden)]
    fn switch(tx_pin: Pin) -> Self::Switch;
}

impl HalfDrive for OpenDrain {
    type Switch = ();
    #[inline(always)]
    fn switch(_: Pin) {}
}

impl HalfDrive for PushPullWhileSending {
    type Switch = DriveSwitch;
    #[inline(always)]
    fn switch(tx_pin: Pin) -> DriveSwitch {
        let (cfgr, cnf0) = gpio::cnf0(tx_pin);
        DriveSwitch { cfgr, cnf0 }
    }
}

/// Separate TX and RX pins.
pub struct FullDuplex<D: FullDrive> {
    /// RX pin bias: `Pull::Up` for a line that floats when the host is
    /// unplugged, `Pull::None` when it is pulled up externally.
    pub rx_pull: Pull,
    pub drive: D,
}

/// One wire: RX muxed onto the TX pin (HDSEL). The wire needs a pull-up,
/// which holds mark while every node listens.
pub struct HalfDuplex<D: HalfDrive> {
    pub drive: D,
}

/// MCU pin arrangement: [`FullDuplex`] or [`HalfDuplex`].
pub trait Wire: Sealed {
    #[doc(hidden)]
    const HALF_DUPLEX: bool;
    #[doc(hidden)]
    type Switch: Switch;
    /// Configure the pins; returns the per-frame switch.
    #[doc(hidden)]
    fn init(&self, tx_pin: Pin, rx_pin: Pin) -> Self::Switch;
}

impl<D: FullDrive> Sealed for FullDuplex<D> {}
impl<D: HalfDrive> Sealed for HalfDuplex<D> {}

impl<D: FullDrive> Wire for FullDuplex<D> {
    const HALF_DUPLEX: bool = false;
    type Switch = ();
    #[inline(always)]
    fn init(&self, tx_pin: Pin, rx_pin: Pin) {
        gpio::configure(tx_pin, D::MODE);
        gpio::configure(rx_pin, PinMode::input_pull(self.rx_pull));
    }
}

impl<D: HalfDrive> Wire for HalfDuplex<D> {
    const HALF_DUPLEX: bool = true;
    type Switch = D::Switch;
    #[inline(always)]
    fn init(&self, tx_pin: Pin, _: Pin) -> D::Switch {
        // RX hears through the parked pin; the pull-up holds mark.
        gpio::configure(tx_pin, PinMode::AF_OPEN_DRAIN);
        D::switch(tx_pin)
    }
}

/// No DE/RE pin.
pub struct NoTxEn;

/// DE/RE pin of an external transceiver or buffer.
#[derive(Copy, Clone)]
pub struct TxEn {
    pub pin: Pin,
    /// Level that puts the transceiver in TX; the inverse selects RX.
    pub tx_level: Level,
}

/// DE/RE pin choice: [`NoTxEn`] or [`TxEn`].
pub trait TxEnable: Sealed {
    #[doc(hidden)]
    type Switch: Switch;
    /// Configure the pin, starting in RX; returns the per-frame switch.
    #[doc(hidden)]
    fn init(&self) -> Self::Switch;
}

impl Sealed for NoTxEn {}
impl Sealed for TxEn {}

impl TxEnable for NoTxEn {
    type Switch = ();
    #[inline(always)]
    fn init(&self) {}
}

impl TxEnable for TxEn {
    type Switch = LevelSwitch;
    #[inline(always)]
    fn init(&self) -> LevelSwitch {
        let rx_level = match self.tx_level {
            Level::High => Level::Low,
            Level::Low => Level::High,
        };
        let (bshr, tx_word) = gpio::level_word(self.pin, self.tx_level);
        let switch = LevelSwitch {
            bshr,
            tx_word,
            rx_word: gpio::level_word(self.pin, rx_level).1,
        };
        rcc::enable_gpio(self.pin.port_index());
        // Latch the RX level before the pin turns output, so it never
        // passes through the reset-default low.
        switch.rx();
        gpio::configure(self.pin, PinMode::OUTPUT_PUSH_PULL);
        switch
    }
}
