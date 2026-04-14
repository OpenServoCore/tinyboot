#![no_std]
#![warn(missing_docs)]

//! Wire protocol for the tinyboot bootloader.
//!
//! Defines the frame format, commands, status codes, and CRC used for
//! host-device communication over UART / RS-485.

/// CRC16-CCITT implementation.
pub mod crc;
/// Frame encoding, decoding, and typed payload access.
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
    let mut result: u16 = 0;
    while i < bytes.len() {
        let d = bytes[i];
        assert!(d >= b'0' && d <= b'9', "non-digit in version string");
        result = result * 10 + (d - b'0') as u16;
        i += 1;
    }
    assert!(result <= 255, "version component exceeds u8");
    result as u8
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

/// Commands (host to device).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cmd {
    /// Query device info (capacity, erase size, versions, mode).
    Info = 0x00,
    /// Erase flash at address. First erase transitions Idle to Updating.
    Erase = 0x01,
    /// Write data at address. Only valid in Updating state.
    Write = 0x02,
    /// Compute CRC16 over app region and transition to Validating.
    Verify = 0x03,
    /// Reset the device. `addr=0`: boot app, `addr=1`: enter bootloader.
    Reset = 0x04,
    /// Flush buffered writes to storage.
    Flush = 0x05,
}

impl Cmd {
    /// Returns true if `b` is a valid command code.
    pub fn is_valid(b: u8) -> bool {
        b <= 0x05
    }
}

/// Response status codes (device to host).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    /// Frame is a request (not a response).
    Request = 0x00,
    /// Success.
    Ok = 0x01,
    /// Flash write or erase failed.
    WriteError = 0x02,
    /// CRC verification failed.
    CrcMismatch = 0x03,
    /// Address or length out of range.
    AddrOutOfBounds = 0x04,
    /// Command not valid in current state.
    Unsupported = 0x05,
    /// Frame payload exceeds maximum size.
    PayloadOverflow = 0x06,
}

impl Status {
    /// Returns true if `b` is a valid status code.
    pub fn is_valid(b: u8) -> bool {
        b <= 0x06
    }
}

/// Transport IO error.
///
/// Returned by [`Frame::read`](frame::Frame::read) when the underlying
/// transport fails. Protocol-level errors (bad CRC, invalid frame) are
/// reported via [`Status`] instead.
#[derive(Debug, PartialEq)]
pub struct ReadError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_is_valid() {
        assert!(Cmd::is_valid(Cmd::Info as u8));
        assert!(Cmd::is_valid(Cmd::Reset as u8));
        assert!(Cmd::is_valid(Cmd::Flush as u8));
        assert!(!Cmd::is_valid(0x06));
        assert!(!Cmd::is_valid(0xFF));
    }

    #[test]
    fn status_is_valid() {
        assert!(Status::is_valid(Status::Request as u8));
        assert!(Status::is_valid(Status::Unsupported as u8));
        assert!(Status::is_valid(Status::PayloadOverflow as u8));
        assert!(!Status::is_valid(0x07));
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
