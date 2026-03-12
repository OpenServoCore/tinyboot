use tiny_boot::traits::{BootState, BootStateStore as TBBootStateStore};

use crate::hal::flash::{self, OB_BASE};

/// OB DATA0 half-word address (DATA0 + nDATA0).
const OB_DATA0: *const u16 = (OB_BASE + 4) as *const u16;

/// Bit layout of the boot state byte (stored in OB DATA0):
///   Bit 7:     Boot request (0 = requested, 1 = idle)
///   Bits 6-4:  State (111=Idle, 011=Updating, 001=Validating, 000=Confirmed)
///   Bits 3-0:  Trial counter (each 0 bit = one trial consumed)
const BR_BIT: u8 = 0x80;
const STATE_MASK: u8 = 0x70;
const TRIAL_MASK: u8 = 0x0F;

#[derive(Debug)]
pub(crate) enum BootStateError {
    WriteError,
}

pub(crate) struct BootStateStore {
    regs: ch32_metapac::flash::Flash,
}

impl BootStateStore {
    pub fn new(regs: ch32_metapac::flash::Flash) -> Self {
        BootStateStore { regs }
    }

    /// Read the raw boot state byte from OB DATA0 (memory-mapped).
    fn read_byte(&self) -> u8 {
        let raw = unsafe { core::ptr::read_volatile(OB_DATA0) };
        let data = raw as u8;
        let inv = (raw >> 8) as u8;
        // If complement check fails, treat as erased (0xFF = Idle)
        if data == !inv { data } else { 0xFF }
    }

    /// Read all 4 OB words from memory-mapped addresses.
    fn read_ob_words(&self) -> [u32; 4] {
        let base = OB_BASE as *const u32;
        core::array::from_fn(|i| unsafe { core::ptr::read_volatile(base.add(i)) })
    }

    /// Build OB words with a modified DATA0 value.
    fn ob_words_with(&self, value: u8) -> [u32; 4] {
        let mut words = self.read_ob_words();
        let half = ((!value as u16) << 8) | (value as u16);
        words[1] = (words[1] & 0xFFFF_0000) | half as u32;
        words
    }

    /// Program a new value into OB DATA0 (1→0 only, no erase).
    fn program_byte(&mut self, value: u8) {
        let words = self.ob_words_with(value);
        flash::unlock_ob(&self.regs);
        flash::write_ob(&self.regs, &words);
        flash::lock_ob(&self.regs);
    }

    /// Erase all OBs and reprogram with modified DATA0.
    fn erase_and_program_byte(&mut self, value: u8) {
        let words = self.ob_words_with(value);
        flash::unlock_ob(&self.regs);
        flash::erase_ob(&self.regs);
        flash::write_ob(&self.regs, &words);
        flash::lock_ob(&self.regs);
    }

    fn decode_state(byte: u8) -> BootState {
        match (byte & STATE_MASK) >> 4 {
            0b111 => BootState::Idle,
            0b011 => BootState::Updating,
            0b001 => BootState::Validating,
            0b000 => BootState::Confirmed,
            // Partially cleared bits — treat as the next valid state
            0b110 | 0b101 | 0b100 => BootState::Updating,
            0b010 => BootState::Validating,
            _ => unreachable!(),
        }
    }
}

impl TBBootStateStore for BootStateStore {
    type Error = BootStateError;

    fn boot_requested(&mut self) -> Result<bool, Self::Error> {
        Ok(self.read_byte() & BR_BIT == 0)
    }

    fn state(&mut self) -> Result<BootState, Self::Error> {
        Ok(Self::decode_state(self.read_byte()))
    }

    fn transition(&mut self) -> Result<BootState, Self::Error> {
        let current = self.read_byte();
        let next_byte = match Self::decode_state(current) {
            BootState::Idle => current & !0x40,
            BootState::Updating => current & !0x20,
            BootState::Validating => current & !0x10,
            BootState::Confirmed => {
                self.erase_and_program_byte(0xFF);
                return Ok(BootState::Idle);
            }
        };
        self.program_byte(next_byte);
        Ok(Self::decode_state(next_byte))
    }

    fn increment_trial(&mut self) -> Result<(), Self::Error> {
        let current = self.read_byte();
        let trials = current & TRIAL_MASK;
        if trials != 0 {
            // Flash can only transition bits from 1→0 (without a page erase), so
            // the trial counter uses one bit per trial: 1111 = 4 remaining, 0111 = 3,
            // 0011 = 2, 0001 = 1, 0000 = exhausted.  Each call clears the highest
            // remaining set bit, effectively consuming one trial.
            let highest_bit = 1 << (trials.count_ones() - 1);
            self.program_byte(current & !highest_bit);
        }
        Ok(())
    }

    fn trials_remaining(&mut self) -> Result<u8, Self::Error> {
        let trials = self.read_byte() & TRIAL_MASK;
        Ok(trials.count_ones() as u8)
    }
}
