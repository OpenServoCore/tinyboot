#![no_std]

pub mod crc;
pub mod frame;
pub(crate) mod sync;

pub use frame::{Data, EraseData, InfoData, MAX_PAYLOAD, VerifyData};

/// Pack a semantic version into a `u16` using 5.5.6 encoding.
///
/// Layout: `(major << 11) | (minor << 6) | patch`
/// - major: 0–31, minor: 0–31, patch: 0–63
/// - `0xFFFF` is reserved as "no version" (erased flash sentinel).
pub const fn pack_version(major: u8, minor: u8, patch: u8) -> u16 {
    ((major as u16) << 11) | ((minor as u16) << 6) | (patch as u16)
}

/// Unpack a 5.5.6-encoded `u16` into `(major, minor, patch)`.
pub const fn unpack_version(v: u16) -> (u8, u8, u8) {
    let major = (v >> 11) as u8 & 0x1F;
    let minor = (v >> 6) as u8 & 0x1F;
    let patch = v as u8 & 0x3F;
    (major, minor, patch)
}

/// `const fn` parse of a `&str` decimal digit sequence into `u8`.
/// Panics at compile time if the string is empty or contains non-digit chars.
pub const fn const_parse_u8(s: &str) -> u8 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut result: u8 = 0;
    while i < bytes.len() {
        let d = bytes[i];
        assert!(d >= b'0' && d <= b'9', "non-digit in version string");
        result = result * 10 + (d - b'0');
        i += 1;
    }
    result
}

/// Expands to `pack_version(MAJOR, MINOR, PATCH)` using the **calling crate's**
/// `Cargo.toml` version fields. Zero runtime cost — evaluates to a `u16` constant.
///
/// Usage: `static VERSION: u16 = tinyboot_protocol::pkg_version!();`
#[macro_export]
macro_rules! pkg_version {
    () => {
        $crate::pack_version(
            $crate::const_parse_u8(env!("CARGO_PKG_VERSION_MAJOR")),
            $crate::const_parse_u8(env!("CARGO_PKG_VERSION_MINOR")),
            $crate::const_parse_u8(env!("CARGO_PKG_VERSION_PATCH")),
        )
    };
}

/// Commands (host → device).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Cmd {
    Info = 0x00,
    Erase = 0x01,
    Write = 0x02,
    Verify = 0x03,
    Reset = 0x04,
}

impl Cmd {
    pub fn is_valid(b: u8) -> bool {
        b <= 0x04
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
    Unsupported = 0x05,
}

impl Status {
    pub fn is_valid(b: u8) -> bool {
        b <= 0x05
    }
}

/// Transport IO error (the only unrecoverable read error).
#[derive(Debug, PartialEq)]
pub struct ReadError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_is_valid() {
        assert!(Cmd::is_valid(Cmd::Info as u8));
        assert!(Cmd::is_valid(Cmd::Reset as u8));
        assert!(!Cmd::is_valid(0x05));
        assert!(!Cmd::is_valid(0xFF));
    }

    #[test]
    fn status_is_valid() {
        assert!(Status::is_valid(Status::Request as u8));
        assert!(Status::is_valid(Status::Unsupported as u8));
        assert!(!Status::is_valid(0x06));
        assert!(!Status::is_valid(0xFF));
    }

    #[test]
    fn pack_unpack_round_trip() {
        assert_eq!(unpack_version(pack_version(0, 0, 1)), (0, 0, 1));
        assert_eq!(unpack_version(pack_version(1, 2, 3)), (1, 2, 3));
        assert_eq!(unpack_version(pack_version(31, 31, 63)), (31, 31, 63));
        assert_eq!(pack_version(0, 0, 0), 0);
    }

    #[test]
    fn erased_flash_sentinel() {
        // 0xFFFF must not collide with any valid version
        let (m, n, p) = unpack_version(0xFFFF);
        assert_eq!((m, n, p), (31, 31, 63));
    }

    #[test]
    fn pkg_version_macro() {
        let v = pkg_version!();
        let expected = pack_version(
            const_parse_u8(env!("CARGO_PKG_VERSION_MAJOR")),
            const_parse_u8(env!("CARGO_PKG_VERSION_MINOR")),
            const_parse_u8(env!("CARGO_PKG_VERSION_PATCH")),
        );
        assert_eq!(v, expected);
    }
}
