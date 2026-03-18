#![no_std]

pub mod crc;
pub mod frame;
pub(crate) mod sync;

/// Commands (host → device).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Cmd {
    Info = 0x01,
    Erase = 0x02,
    Write = 0x03,
    Verify = 0x04,
    Reset = 0x05,
}

impl Cmd {
    pub fn is_valid(&self) -> bool {
        let b = unsafe { *(self as *const Self as *const u8) };
        (0x01..=0x05).contains(&b)
    }
}

/// Response status codes (device → host).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Status {
    Request = 0x00,
    Ok = 0x01,
    WriteError = 0x02,
    CrcMismatch = 0x03,
    AddrOutOfBounds = 0x04,
}

impl Status {
    pub fn is_valid(&self) -> bool {
        let b = unsafe { *(self as *const Self as *const u8) };
        b <= 0x04
    }
}

/// Frame parse/validation error.
#[derive(Debug, PartialEq)]
pub enum ReadError {
    /// Transport IO error.
    Io,
    /// CRC mismatch.
    Crc,
    /// Invalid command or status byte.
    InvalidFrame,
    /// Data payload exceeds buffer size.
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_is_valid() {
        assert!(Cmd::Info.is_valid());
        assert!(Cmd::Reset.is_valid());
    }

    #[test]
    fn cmd_discriminant_range() {
        assert!(!(0x01..=0x05).contains(&0x00u8));
        assert!(!(0x01..=0x05).contains(&0x06u8));
        assert!((0x01..=0x05).contains(&(Cmd::Info as u8)));
        assert!((0x01..=0x05).contains(&(Cmd::Reset as u8)));
    }
}
