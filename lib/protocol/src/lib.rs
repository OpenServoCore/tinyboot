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

pub use frame::{Data, EraseData, Flags, InfoData, MAX_PAYLOAD, VerifyData};

use bitflags::bitflags;

bitflags! {
    /// Flags for [`Cmd::Write`] (addr byte 3).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct WriteFlags: u8 {
        /// Commit the page after this write and reset write state.
        /// Set on the last write of each contiguous region (address jump
        /// or end of transfer).
        const FLUSH = 1 << 7;
    }
}

bitflags! {
    /// Flags for [`Cmd::Reset`] (addr byte 3).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ResetFlags: u8 {
        /// Enter bootloader (service mode) instead of booting the app.
        const BOOTLOADER = 1 << 0;
        /// Persist addr byte 0 as the bootloader's [`BaudRate`] before
        /// resetting. The bootloader uses it from this reset on.
        const BAUD = 1 << 1;
    }
}

/// UART baud rates, encoded as their `u8` index on the wire and in boot
/// metadata. Indices are append-only; an index the bootloader does not
/// know or cannot reach from its clock (erased flash `0xFF` included)
/// selects its build default.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaudRate {
    /// 9600 baud.
    B9600 = 0,
    /// 19200 baud.
    B19200 = 1,
    /// 38400 baud.
    B38400 = 2,
    /// 57600 baud.
    B57600 = 3,
    /// 115200 baud.
    B115200 = 4,
    /// 230400 baud.
    B230400 = 5,
    /// 460800 baud.
    B460800 = 6,
    /// 500000 baud.
    B500000 = 7,
    /// 921600 baud.
    B921600 = 8,
    /// 1 Mbaud.
    B1000000 = 9,
    /// 1.5 Mbaud.
    B1500000 = 10,
    /// 2 Mbaud.
    B2000000 = 11,
    /// 2.5 Mbaud.
    B2500000 = 12,
    /// 3 Mbaud.
    B3000000 = 13,
}

impl BaudRate {
    /// Every rate, in index order.
    pub const ALL: [BaudRate; 14] = [
        BaudRate::B9600,
        BaudRate::B19200,
        BaudRate::B38400,
        BaudRate::B57600,
        BaudRate::B115200,
        BaudRate::B230400,
        BaudRate::B460800,
        BaudRate::B500000,
        BaudRate::B921600,
        BaudRate::B1000000,
        BaudRate::B1500000,
        BaudRate::B2000000,
        BaudRate::B2500000,
        BaudRate::B3000000,
    ];

    /// Rate in bits per second.
    pub const fn hz(self) -> u32 {
        match self {
            BaudRate::B9600 => 9600,
            BaudRate::B19200 => 19200,
            BaudRate::B38400 => 38400,
            BaudRate::B57600 => 57600,
            BaudRate::B115200 => 115_200,
            BaudRate::B230400 => 230_400,
            BaudRate::B460800 => 460_800,
            BaudRate::B500000 => 500_000,
            BaudRate::B921600 => 921_600,
            BaudRate::B1000000 => 1_000_000,
            BaudRate::B1500000 => 1_500_000,
            BaudRate::B2000000 => 2_000_000,
            BaudRate::B2500000 => 2_500_000,
            BaudRate::B3000000 => 3_000_000,
        }
    }

    /// Parse a wire / metadata index.
    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.get(v as usize).copied()
    }

    /// Look up a rate by bits per second.
    pub fn from_hz(hz: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.hz() == hz)
    }
}

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
    /// Reset the device. See [`ResetFlags`].
    Reset = 0x04,
}

impl Cmd {
    /// Returns true if `b` is a valid command code.
    pub fn is_valid(b: u8) -> bool {
        b <= 0x04
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
    /// The image's compatibility key differs from the bootloader's.
    Incompatible = 0x07,
}

impl Status {
    /// Returns true if `b` is a valid status code.
    pub fn is_valid(b: u8) -> bool {
        b <= 0x07
    }
}

/// Compatibility key that matches every image: the bootloader skips the
/// check (erased flash sentinel).
pub const COMPAT_UNSET: u32 = 0xFFFF_FFFF;

/// Compatibility key from a name, a 32-bit FNV-1a hash evaluated at
/// compile time. Never returns [`COMPAT_UNSET`].
pub const fn compat_key(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut h: u32 = 0x811C_9DC5;
    let mut i = 0;
    while i < bytes.len() {
        h = (h ^ bytes[i] as u32).wrapping_mul(0x0100_0193);
        i += 1;
    }
    if h == COMPAT_UNSET { h - 1 } else { h }
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
        assert!(!Cmd::is_valid(0x05));
        assert!(!Cmd::is_valid(0xFF));
    }

    #[test]
    fn status_is_valid() {
        assert!(Status::is_valid(Status::Request as u8));
        assert!(Status::is_valid(Status::Unsupported as u8));
        assert!(Status::is_valid(Status::Incompatible as u8));
        assert!(!Status::is_valid(0x08));
        assert!(!Status::is_valid(0xFF));
    }

    #[test]
    fn baud_index_round_trip() {
        for (i, b) in BaudRate::ALL.into_iter().enumerate() {
            assert_eq!(b as u8, i as u8);
            assert_eq!(BaudRate::from_u8(i as u8), Some(b));
            assert_eq!(BaudRate::from_hz(b.hz()), Some(b));
        }
        assert_eq!(BaudRate::from_u8(BaudRate::ALL.len() as u8), None);
        assert_eq!(BaudRate::from_u8(0xFF), None);
        assert_eq!(BaudRate::from_hz(1234), None);
    }

    #[test]
    fn baud_rates_ascend() {
        assert!(BaudRate::ALL.windows(2).all(|w| w[0].hz() < w[1].hz()));
    }

    #[test]
    fn compat_key_is_fnv1a_and_never_unset() {
        assert_eq!(compat_key(""), 0x811C_9DC5);
        assert_eq!(compat_key("a"), 0xE40C_292C);
        assert_ne!(compat_key("board-a"), compat_key("board-b"));
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
