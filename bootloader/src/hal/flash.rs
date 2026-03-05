use core::convert::Infallible;

use embedded_storage::{
    ReadStorage,
    nor_flash::{ErrorType, NorFlash, ReadNorFlash},
};

use crate::hal::common::app_flash_size;

pub(crate) struct Ch32Flash;

impl Ch32Flash {
    pub fn new() -> Self {
        Ch32Flash
    }
}

impl ErrorType for Ch32Flash {
    type Error = Infallible;
}

impl NorFlash for Ch32Flash {
    const WRITE_SIZE: usize = 64;
    const ERASE_SIZE: usize = 1024;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        todo!()
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }
}

impl ReadNorFlash for Ch32Flash {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn capacity(&self) -> usize {
        app_flash_size()
    }
}

impl ReadStorage for Ch32Flash {
    type Error = ();

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn capacity(&self) -> usize {
        todo!()
    }
}
