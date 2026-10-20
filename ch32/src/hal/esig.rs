//! ESIG electronic signature: the factory-programmed 96-bit unique id in
//! system flash, at the same address on every supported family.

const UNIID: *const u8 = 0x1FFF_F7E8 as *const u8;

/// The chip's 96-bit unique id, 12 bytes least significant first.
#[inline(always)]
pub fn uid() -> &'static [u8; 12] {
    // SAFETY: ESIG is always-readable factory flash, at any access width.
    unsafe { &*(UNIID as *const [u8; 12]) }
}
