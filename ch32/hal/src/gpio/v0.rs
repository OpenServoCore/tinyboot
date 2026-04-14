use core::convert::Infallible;

use crate::Pin;

#[derive(Copy, Clone)]
pub enum Pull {
    None,
    Up,
    Down,
}

/// GPIO pin configuration.
///
/// Encodes the 4-bit CFGLR field `[MODE(2) | CNF(2)]` directly.
/// Bit 7 = set ODR high, bit 6 = set ODR low, bits 3:0 = CFGLR nibble.
#[derive(Copy, Clone)]
pub struct PinMode(u8);

// MODE bits: INPUT=0b00, OUTPUT_10MHZ=0b01
// CNF bits (shifted <<2): ANALOG/PP=0b0000, FLOAT/OD=0b0100, PULL/AF_PP=0b1000, AF_OD=0b1100
impl PinMode {
    pub const INPUT_FLOATING: Self = Self(0b0100); // MODE=00 CNF=01
    pub const INPUT_PULL_UP: Self = Self(0b1000 | 0x80); // MODE=00 CNF=10, ODR=1
    pub const INPUT_PULL_DOWN: Self = Self(0b1000 | 0x40); // MODE=00 CNF=10, ODR=0
    pub const OUTPUT_PUSH_PULL: Self = Self(0b0001); // MODE=01 CNF=00
    pub const OUTPUT_OPEN_DRAIN: Self = Self(0b0101); // MODE=01 CNF=01
    pub const AF_PUSH_PULL: Self = Self(0b1001); // MODE=01 CNF=10
    pub const AF_OPEN_DRAIN: Self = Self(0b1101); // MODE=01 CNF=11

    pub fn input_pull(pull: Pull) -> Self {
        match pull {
            Pull::Up => Self::INPUT_PULL_UP,
            Pull::Down => Self::INPUT_PULL_DOWN,
            Pull::None => Self::INPUT_FLOATING,
        }
    }
}

#[inline(always)]
pub fn configure(pin: Pin, mode: PinMode) {
    let regs = pin.gpio_regs();
    let n = pin.pin_number();

    let shift = n * 4;
    let mask = !(0xFu32 << shift);
    let bits = ((mode.0 & 0x0F) as u32) << shift;
    let prev = regs.cfglr().read().0;
    regs.cfglr()
        .write_value(ch32_metapac::gpio::regs::Cfglr(prev & mask | bits));

    if mode.0 & 0xC0 != 0 {
        let mut outdr = regs.outdr().read();
        outdr.set_odr(n, mode.0 & 0x80 != 0);
        regs.outdr().write_value(outdr);
    }
}

pub fn set_high(pin: Pin) {
    pin.gpio_regs()
        .bshr()
        .write(|w| w.0 = 1 << pin.pin_number());
}

pub fn set_low(pin: Pin) {
    pin.gpio_regs().bcr().write(|w| w.0 = 1 << pin.pin_number());
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
