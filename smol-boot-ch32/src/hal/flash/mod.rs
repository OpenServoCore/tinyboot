use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

use crate::hal::common::{APP_BASE, APP_PTR, APP_SIZE, FLASH_ERASE_SIZE, FLASH_WRITE_SIZE};

#[cfg_attr(flash_v0, path = "v0.rs")]
mod family;

#[derive(Debug)]
pub enum FlashError {
    NotAligned,
    OutOfBounds,
    Protected,
}

impl NorFlashError for FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            FlashError::NotAligned => NorFlashErrorKind::NotAligned,
            FlashError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            FlashError::Protected => NorFlashErrorKind::Other,
        }
    }
}

pub(crate) struct Flash {
    regs: ch32_metapac::flash::Flash,
}

struct FlashUnlocked<'a> {
    regs: &'a ch32_metapac::flash::Flash,
}

impl Drop for FlashUnlocked<'_> {
    fn drop(&mut self) {
        family::lock(self.regs);
    }
}

impl Flash {
    pub fn new(regs: ch32_metapac::flash::Flash) -> Self {
        Flash { regs }
    }

    fn unlock(&self) -> FlashUnlocked<'_> {
        family::unlock(&self.regs);
        FlashUnlocked { regs: &self.regs }
    }
}

impl ErrorType for Flash {
    type Error = FlashError;
}

impl NorFlash for Flash {
    const WRITE_SIZE: usize = FLASH_WRITE_SIZE;
    const ERASE_SIZE: usize = FLASH_ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if from as usize % FLASH_ERASE_SIZE != 0 || to as usize % FLASH_ERASE_SIZE != 0 {
            return Err(FlashError::NotAligned);
        }
        if to as usize > APP_SIZE {
            return Err(FlashError::OutOfBounds);
        }
        let _guard = self.unlock();
        let mut addr = APP_BASE + from;
        let end = APP_BASE + to;
        while addr < end {
            family::erase_page(&self.regs, addr)?;
            addr += FLASH_ERASE_SIZE as u32;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        if offset as usize % FLASH_WRITE_SIZE != 0 || bytes.len() % FLASH_WRITE_SIZE != 0 {
            return Err(FlashError::NotAligned);
        }
        if offset as usize + bytes.len() > APP_SIZE {
            return Err(FlashError::OutOfBounds);
        }
        let _guard = self.unlock();
        let mut addr = APP_BASE + offset;
        for chunk in bytes.chunks_exact(FLASH_WRITE_SIZE) {
            family::write_page(&self.regs, addr, chunk)?;
            addr += FLASH_WRITE_SIZE as u32;
        }
        Ok(())
    }
}

impl ReadNorFlash for Flash {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        if offset as usize + bytes.len() > APP_SIZE {
            return Err(FlashError::OutOfBounds);
        }
        let flash = unsafe { core::slice::from_raw_parts(APP_PTR as *const u8, APP_SIZE) };
        let offset = offset as usize;
        bytes.copy_from_slice(&flash[offset..offset + bytes.len()]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        APP_SIZE
    }
}
