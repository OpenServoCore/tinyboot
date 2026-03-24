use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use tinyboot::traits::boot::Storage as StorageTrait;

use tinyboot_ch32_hal::flash::FlashWriter;

const FLASH_WRITE_SIZE: usize = 2;
const FLASH_ERASE_SIZE: usize = 64;

/// Flash storage configuration.
pub struct StorageConfig {
    /// Physical base address of the app region.
    pub app_base: u32,
    /// Size of the app region in bytes.
    pub app_size: usize,
}

#[derive(Debug)]
pub enum StorageError {
    NotAligned,
    OutOfBounds,
    Protected,
}

impl NorFlashError for StorageError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            StorageError::NotAligned => NorFlashErrorKind::NotAligned,
            StorageError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            StorageError::Protected => NorFlashErrorKind::Other,
        }
    }
}

/// CH32 flash storage implementing [`NorFlash`] and the tinyboot [`Storage`](tinyboot::traits::boot::Storage) trait.
pub struct Storage {
    app_base: u32,
    app_size: usize,
}

impl Storage {
    /// Create storage from configuration.
    #[inline(always)]
    pub fn new(config: StorageConfig) -> Self {
        Storage {
            app_base: config.app_base,
            app_size: config.app_size,
        }
    }

    fn app_ptr(&self) -> *const u8 {
        self.app_base as *const u8
    }
}

impl ErrorType for Storage {
    type Error = StorageError;
}

impl NorFlash for Storage {
    const WRITE_SIZE: usize = FLASH_WRITE_SIZE;
    const ERASE_SIZE: usize = FLASH_ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if !(from as usize).is_multiple_of(FLASH_ERASE_SIZE)
            || !(to as usize).is_multiple_of(FLASH_ERASE_SIZE)
        {
            return Err(StorageError::NotAligned);
        }
        if to as usize > self.app_size {
            return Err(StorageError::OutOfBounds);
        }
        let writer = FlashWriter::standard();
        writer.erase_start();
        let mut addr = self.app_base + from;
        let end = self.app_base + to;
        while addr < end {
            writer.erase(addr);
            addr += FLASH_ERASE_SIZE as u32;
        }
        writer.operation_end();
        // Write-protection check is debug-only: unlock() disables protection
        // before the protocol loop, so WRPRTERR should never fire in a correctly
        // configured system. The verify step catches silent write failures.
        // Keeping this out of release saves ~40-60 bytes against the 1920-byte budget.
        #[cfg(debug_assertions)]
        if writer.check_wrprterr() {
            return Err(StorageError::Protected);
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        if !(offset as usize).is_multiple_of(FLASH_WRITE_SIZE)
            || !bytes.len().is_multiple_of(FLASH_WRITE_SIZE)
        {
            return Err(StorageError::NotAligned);
        }
        if offset as usize + bytes.len() > self.app_size {
            return Err(StorageError::OutOfBounds);
        }
        let writer = FlashWriter::standard();
        writer.write_start();
        let mut addr = self.app_base + offset;
        for pair in bytes.chunks_exact(2) {
            let halfword = u16::from_le_bytes([pair[0], pair[1]]);
            writer.write(addr, halfword);
            addr += 2;
        }
        writer.operation_end();

        // See erase() for rationale on debug-only write-protection check.
        #[cfg(debug_assertions)]
        if writer.check_wrprterr() {
            return Err(StorageError::Protected);
        }
        Ok(())
    }
}

impl StorageTrait for Storage {
    fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.app_ptr(), self.app_size) }
    }

    fn unlock(&mut self) {
        tinyboot_ch32_hal::flash::unlock();
    }
}

impl ReadNorFlash for Storage {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        if offset as usize + bytes.len() > self.app_size {
            return Err(StorageError::OutOfBounds);
        }
        let src = unsafe { core::slice::from_raw_parts(self.app_ptr(), self.app_size) };
        let offset = offset as usize;
        bytes.copy_from_slice(&src[offset..offset + bytes.len()]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.app_size
    }
}
