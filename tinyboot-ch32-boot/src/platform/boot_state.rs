use core::slice::{from_raw_parts, from_raw_parts_mut};

use tinyboot::traits::BootState;
use tinyboot::traits::boot::BootMetaStore as TBBootMetaStore;
use tinyboot_ch32_hal::flash;

#[derive(Debug)]
pub enum BootMetaError {
    InvalidTransition,
    TrialsExhausted,
}

/// CH32 boot metadata stored in user flash.
///
/// Address defined by `__tinyboot_meta_start` linker symbol (see memory.x).
#[repr(C)]
pub struct BootMetaStore {
    state: u8,
    trials: u8,
    checksum: u16,
    app_size: u32,
}

impl Default for BootMetaStore {
    /// Create a cached instance of boot metadata by reading from user flash.
    // TODO: unlock here because `step_down()` may write before
    // `Storage::unlock()` runs in the protocol loop. Move to a
    // proper HAL init path when one exists.
    #[inline(always)]
    fn default() -> Self {
        flash::unlock();
        unsafe { core::ptr::read_volatile(flash::meta_addr() as *const Self) }
    }
}

impl BootMetaStore {
    fn as_bytes(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { from_raw_parts_mut(self as *mut Self as *mut u8, size_of::<Self>()) }
    }

    /// Erase meta page and rewrite cached metadata via FTPG.
    fn write(&self) {
        let addr = flash::meta_addr();
        flash::erase(addr);
        flash::write(addr, self.as_bytes());
    }

    /// Bit-clear step down on a single byte. Updates cache + flash via FTPG.
    fn step_down(&mut self, offset: usize, floor: u8) -> Option<u8> {
        let bytes = self.as_bytes_mut();
        if bytes[offset] <= floor {
            return None;
        }
        bytes[offset] &= bytes[offset] >> 1;
        flash::write(flash::meta_addr(), bytes);
        Some(bytes[offset])
    }
}

impl TBBootMetaStore for BootMetaStore {
    type Error = BootMetaError;

    fn boot_state(&self) -> BootState {
        BootState::from_u8(self.state)
    }

    fn has_trials(&self) -> bool {
        self.trials != 0
    }

    fn app_checksum(&self) -> u16 {
        self.checksum
    }

    fn app_size(&self) -> u32 {
        self.app_size
    }

    fn advance(&mut self) -> Result<BootState, Self::Error> {
        let next = self
            .step_down(0, BootState::Validating as u8)
            .ok_or(BootMetaError::InvalidTransition)?;
        Ok(BootState::from_u8(next))
    }

    fn consume_trial(&mut self) -> Result<(), Self::Error> {
        self.step_down(1, 0).ok_or(BootMetaError::TrialsExhausted)?;
        Ok(())
    }

    fn refresh(
        &mut self,
        checksum: u16,
        state: BootState,
        app_size: u32,
    ) -> Result<(), Self::Error> {
        self.state = state as u8;
        self.trials = 0xFF;
        self.checksum = checksum;
        self.app_size = app_size;
        self.write();
        Ok(())
    }
}
