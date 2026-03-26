use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use tinyboot::traits::boot::Storage as StorageTrait;

use tinyboot_ch32_hal::flash::{BUF_LOAD_SIZE, FlashWriter, PAGE_SIZE};

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
    const WRITE_SIZE: usize = PAGE_SIZE;
    const ERASE_SIZE: usize = PAGE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        debug_assert!(
            (from as usize).is_multiple_of(PAGE_SIZE) && (to as usize).is_multiple_of(PAGE_SIZE),
            "erase alignment: from={from}, to={to}"
        );
        debug_assert!(to as usize <= self.app_size, "erase out of bounds");
        let writer = FlashWriter::usr();
        writer.erase_start();
        let mut addr = self.app_base + from;
        let end = self.app_base + to;
        while addr < end {
            writer.erase(addr);
            addr += PAGE_SIZE as u32;
        }
        writer.operation_end();
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        debug_assert!(
            (offset as usize).is_multiple_of(PAGE_SIZE)
                && bytes.len() <= PAGE_SIZE
                && bytes.len().is_multiple_of(BUF_LOAD_SIZE),
            "write alignment: offset={offset}, len={}",
            bytes.len()
        );
        debug_assert!(
            offset as usize + bytes.len() <= self.app_size,
            "write out of bounds"
        );
        let page_addr = self.app_base + offset;
        let writer = FlashWriter::usr();
        writer.write_start();
        writer.fast_write_buf_reset();
        let mut addr = page_addr;
        let mut ptr = bytes.as_ptr() as *const u32;
        for _ in 0..bytes.len() / BUF_LOAD_SIZE {
            // SAFETY: ptr advances within data bounds; RingBuf is repr(align(4)).
            let word = unsafe { ptr.read() };
            writer.fast_write_buf_load(addr, word);
            addr += BUF_LOAD_SIZE as u32;
            ptr = unsafe { ptr.add(1) };
        }
        writer.fast_write_page_program(page_addr);
        writer.operation_end();
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
