use tiny_boot::traits::{BootMeta, BootMetaStore as TBBootMetaStore, BootState};

use crate::common::META_BASE;
use crate::hal::flash::FlashWriter;

/// Memory-mapped read pointer for the boot meta struct.
const META_PTR: *const BootMeta = META_BASE as *const BootMeta;

/// Byte offset of the `state` field within `BootMeta`.
const STATE_OFFSET: u32 = 0;

/// Byte offset of the `trials` field within `BootMeta`.
const TRIALS_OFFSET: u32 = 2;

#[derive(Debug)]
pub(crate) enum BootMetaError {
    InvalidTransition,
    TrialsExhausted,
}

pub(crate) struct BootMetaStore {
    regs: ch32_metapac::flash::Flash,
}

impl BootMetaStore {
    pub fn new(regs: ch32_metapac::flash::Flash) -> Self {
        BootMetaStore { regs }
    }

    /// Write a u16 at a half-word-aligned offset within the meta region (1→0 only).
    fn patch_u16(&mut self, offset: u32, value: u16) {
        let writer = FlashWriter::standard(&self.regs);
        writer.write_halfword(META_BASE + offset, value);
    }

    /// Read a u16 at a half-word-aligned offset within the meta region.
    fn read_u16(&self, offset: u32) -> u16 {
        unsafe { core::ptr::read_volatile((META_BASE + offset) as *const u16) }
    }

    /// Clear the MSB of a contiguous-ones field: `v & (v >> 1)`.
    /// Returns the new value, or `None` if already at or below `floor`.
    fn step_down(&mut self, offset: u32, floor: u16) -> Option<u16> {
        let current = self.read_u16(offset);
        if current <= floor {
            return None;
        }
        let next = current & (current >> 1);
        self.patch_u16(offset, next);
        Some(next)
    }
}

impl TBBootMetaStore for BootMetaStore {
    type Error = BootMetaError;

    fn read(&self) -> BootMeta {
        unsafe { core::ptr::read_volatile(META_PTR) }
    }

    fn advance(&mut self) -> Result<BootState, Self::Error> {
        let next = self
            .step_down(STATE_OFFSET, BootState::Confirmed as u16)
            .ok_or(BootMetaError::InvalidTransition)?;
        Ok(BootState::from_u16(next))
    }

    fn consume_trial(&mut self) -> Result<(), Self::Error> {
        self.step_down(TRIALS_OFFSET, 0)
            .ok_or(BootMetaError::TrialsExhausted)?;
        Ok(())
    }
}
