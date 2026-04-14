use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use tinyboot::traits::boot::Storage as StorageTrait;

use tinyboot_ch32_hal::flash::{self, PAGE_SIZE};

#[derive(Debug)]
pub enum StorageError {
    NotAligned,
    OutOfBounds,
}

impl NorFlashError for StorageError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            StorageError::NotAligned => NorFlashErrorKind::NotAligned,
            StorageError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
        }
    }
}

/// CH32 flash storage implementing [`NorFlash`] and the tinyboot [`Storage`](tinyboot::traits::boot::Storage) trait.
///
/// Geometry comes from linker symbols (`__tb_app_base`, `__tb_meta_base`),
/// cached in struct fields to avoid repeated address loads.
pub struct Storage {
    app_base: u32,
    app_size: usize,
}

impl Default for Storage {
    /// Reads `__tb_app_base` and `__tb_meta_base` linker symbols to determine the app region.
    #[inline(always)]
    fn default() -> Self {
        let app_base = flash::app_base();
        let app_size = (flash::meta_addr() - app_base) as usize;
        Storage { app_base, app_size }
    }
}

impl Storage {
    fn app_ptr(&self) -> *const u8 {
        self.app_base as *const u8
    }
}

impl ErrorType for Storage {
    type Error = StorageError;
}

impl NorFlash for Storage {
    const WRITE_SIZE: usize = PAGE_SIZE;
    const ERASE_SIZE: usize = PAGE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        debug_assert!(
            (from as usize).is_multiple_of(PAGE_SIZE) && (to as usize).is_multiple_of(PAGE_SIZE),
            "erase alignment: from={}, to={}",
            from,
            to
        );
        debug_assert!(to as usize <= self.app_size, "erase out of bounds");
        let mut addr = self.app_base + from;
        let end = self.app_base + to;
        while addr < end {
            flash::erase(addr);
            addr += PAGE_SIZE as u32;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        debug_assert!(
            (offset as usize).is_multiple_of(PAGE_SIZE) && bytes.len() <= PAGE_SIZE,
            "write alignment: offset={}, len={}",
            offset,
            bytes.len()
        );
        debug_assert!(
            offset as usize + bytes.len() <= self.app_size,
            "write out of bounds"
        );
        flash::write(self.app_base + offset, bytes);
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
