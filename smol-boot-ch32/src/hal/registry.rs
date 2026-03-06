use smol_boot::hal::{Registry, RegistryKey};

const OB_DATA0: *const u16 = 0x1FFFF804 as *const u16;

/// CH32 Bootloader Registry implementation using Optional User Bytes.
/// This works on all CH32 devices
pub(crate) struct Ch32Registry;

impl Ch32Registry {
    pub fn new() -> Self {
        Ch32Registry {}
    }
}

pub(crate) enum Ch32RegistryError {
    UnitializedValue,
}

impl Registry for Ch32Registry {
    type Error = Ch32RegistryError;

    fn read(&mut self, key: RegistryKey) -> Result<u8, Self::Error> {
        match key {
            RegistryKey::BootRequest => read_ob(OB_DATA0),
        }
    }

    fn write(&mut self, _key: RegistryKey, _value: u8) -> Result<(), Self::Error> {
        todo!()
    }
}

fn read_ob(addr: *const u16) -> Result<u8, Ch32RegistryError> {
    let raw = unsafe { core::ptr::read_volatile(addr) };
    let inv = (raw >> 8) as u8;
    let data = raw as u8;
    if data == !inv {
        Ok(data)
    } else {
        Err(Ch32RegistryError::UnitializedValue)
    }
}
