use core::convert::Infallible;

use ch32_metapac::gpio::vals::{Cnf, Mode};

use crate::Pin;

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub enum Pull {
    None,
    Up,
    Down,
}

#[allow(dead_code)]
pub enum PinMode {
    InputFloating,
    InputPull(Pull),
    OutputPushPull,
    OutputOpenDrain,
    AfPushPull,
    AfOpenDrain,
}

#[inline(always)]
pub fn configure(pin: Pin, mode: PinMode) {
    let regs = pin.gpio_regs();
    let n = pin.pin_number();

    let (m, cnf, odr) = match mode {
        PinMode::InputFloating => (Mode::INPUT, Cnf::FLOATING_IN__OPEN_DRAIN_OUT, None),
        PinMode::InputPull(Pull::Up) => (Mode::INPUT, Cnf::PULL_IN__AF_PUSH_PULL_OUT, Some(true)),
        PinMode::InputPull(Pull::Down) => {
            (Mode::INPUT, Cnf::PULL_IN__AF_PUSH_PULL_OUT, Some(false))
        }
        PinMode::InputPull(Pull::None) => (Mode::INPUT, Cnf::FLOATING_IN__OPEN_DRAIN_OUT, None),
        PinMode::OutputPushPull => (Mode::OUTPUT_10MHZ, Cnf::ANALOG_IN__PUSH_PULL_OUT, None),
        PinMode::OutputOpenDrain => (Mode::OUTPUT_10MHZ, Cnf::FLOATING_IN__OPEN_DRAIN_OUT, None),
        PinMode::AfPushPull => (Mode::OUTPUT_10MHZ, Cnf::PULL_IN__AF_PUSH_PULL_OUT, None),
        PinMode::AfOpenDrain => (Mode::OUTPUT_10MHZ, Cnf::AF_OPEN_DRAIN_OUT, None),
    };

    // Direct read-modify-write to avoid closure extraction.
    // Each pin occupies 4 bits in CFGLR: [MODE(2) | CNF(2)] at offset n*4.
    let shift = n * 4;
    let mask = !(0xFu32 << shift);
    let bits = ((m.to_bits() as u32) | ((cnf.to_bits() as u32) << 2)) << shift;
    let prev = regs.cfglr().read().0;
    regs.cfglr()
        .write_value(ch32_metapac::gpio::regs::Cfglr(prev & mask | bits));

    if let Some(val) = odr {
        let mut outdr = regs.outdr().read();
        outdr.set_odr(n, val);
        regs.outdr().write_value(outdr);
    }
}

pub fn set_high(pin: Pin) {
    pin.gpio_regs()
        .bshr()
        .write(|w| w.set_bs(pin.pin_number(), true));
}

pub fn set_low(pin: Pin) {
    pin.gpio_regs()
        .bcr()
        .write(|w| w.set_br(pin.pin_number(), true));
}

impl embedded_hal::digital::ErrorType for Pin {
    type Error = Infallible;
}

impl embedded_hal::digital::OutputPin for Pin {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        set_high(*self);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        set_low(*self);
        Ok(())
    }
}
