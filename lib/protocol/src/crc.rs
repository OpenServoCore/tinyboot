/// CRC16 initial value.
pub const CRC_INIT: u16 = 0xFFFF;

/// CRC16-CCITT (poly 0x1021, init 0xFFFF).
///
/// Bit-bang implementation — no lookup table, minimal flash footprint.
/// Call incrementally by passing the previous return value as `crc`:
///
/// ```
/// # use tinyboot_protocol::crc::crc16;
/// let mut crc = 0xFFFF;
/// crc = crc16(crc, b"1234");
/// crc = crc16(crc, b"56789");
/// assert_eq!(crc, 0x29B1);
/// ```
pub fn crc16(mut crc: u16, data: &[u8]) -> u16 {
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_value() {
        assert_eq!(crc16(0xFFFF, b"123456789"), 0x29B1);
    }

    #[test]
    fn incremental() {
        let mut crc = 0xFFFF;
        crc = crc16(crc, b"1234");
        crc = crc16(crc, b"56789");
        assert_eq!(crc, 0x29B1);
    }

    #[test]
    fn empty() {
        assert_eq!(crc16(0xFFFF, b""), 0xFFFF);
    }

    #[test]
    fn single_byte() {
        // Known value: CRC16-CCITT of [0x00] with init 0xFFFF
        let crc = crc16(0xFFFF, &[0x00]);
        assert_eq!(crc, 0xE1F0);
    }
}
