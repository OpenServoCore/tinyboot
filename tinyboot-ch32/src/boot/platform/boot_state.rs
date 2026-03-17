use tinyboot::traits::{BootMeta, BootMetaStore as TBBootMetaStore, BootState};

use crate::hal::flash::FlashWriter;

const STATE_OFFSET: u32 = 0;
const TRIALS_OFFSET: u32 = 2;

pub struct MetaConfig {
    pub meta_base: u32,
}

#[derive(Debug)]
pub enum BootMetaError {
    InvalidTransition,
    TrialsExhausted,
}

pub struct BootMetaStore {
    meta_base: u32,
}

impl BootMetaStore {
    pub fn new(config: MetaConfig) -> Self {
        BootMetaStore {
            meta_base: config.meta_base,
        }
    }

    fn meta_ptr(&self) -> *const BootMeta {
        self.meta_base as *const BootMeta
    }

    fn patch_u16(&mut self, offset: u32, value: u16) {
        let writer = FlashWriter::standard();
        writer.write_halfword(self.meta_base + offset, value);
    }

    fn read_u16(&self, offset: u32) -> u16 {
        unsafe { core::ptr::read_volatile((self.meta_base + offset) as *const u16) }
    }

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
        unsafe { core::ptr::read_volatile(self.meta_ptr()) }
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
