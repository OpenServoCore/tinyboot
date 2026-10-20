//! Sharing a bus: [`Unaddressed`] answers every frame, [`Addressable`]
//! answers only once a host selects it by its unique id.

use tinyboot_protocol::Status;
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::Frame;

/// How a device shares its bus. `Copy`, so the bootloader hands it to its
/// dispatcher without moving the platform.
pub trait Addressing: Copy {
    /// `false` for [`Unaddressed`]; `Select` is then unsupported and every
    /// frame is answered.
    const ADDRESSABLE: bool;

    /// Unique id, 1 to [`MAX_UID_LEN`] bytes, least significant first.
    fn uid(&self) -> &'static [u8];

    /// Whether this device answers commands other than `Select`.
    fn selected(&self) -> bool;

    /// Select or deselect this device.
    fn set_selected(&mut self, selected: bool);
}

/// One device per bus: every frame is answered, as without addressing.
#[derive(Clone, Copy)]
pub struct Unaddressed;

impl Addressing for Unaddressed {
    const ADDRESSABLE: bool = false;

    #[inline(always)]
    fn uid(&self) -> &'static [u8] {
        &[]
    }

    #[inline(always)]
    fn selected(&self) -> bool {
        true
    }

    #[inline(always)]
    fn set_selected(&mut self, _: bool) {}
}

/// Many devices per bus. A device answers a `Select` whose prefix its
/// unique id starts with, and stays silent to everything else until such a
/// `Select` selects it. Selection is in RAM, so every reset starts
/// deselected.
/// `F` returns the id; a function item such as `chip_uid` keeps the
/// device's state to the one `selected` byte and lets the id's address and
/// length fold into the code.
#[derive(Clone, Copy)]
pub struct Addressable<const L: usize, F: Fn() -> &'static [u8; L] + Copy> {
    uid: F,
    selected: bool,
}

impl<const L: usize, F: Fn() -> &'static [u8; L] + Copy> Addressable<L, F> {
    /// Address the device by the id `uid` returns (1 to [`MAX_UID_LEN`]
    /// bytes, least significant first), typically the chip's factory
    /// unique id.
    #[inline(always)]
    pub const fn new(uid: F) -> Self {
        const { assert!(L >= 1 && L <= MAX_UID_LEN) };
        Self {
            uid,
            selected: false,
        }
    }
}

impl<const L: usize, F: Fn() -> &'static [u8; L] + Copy> Addressing for Addressable<L, F> {
    const ADDRESSABLE: bool = true;

    #[inline(always)]
    fn uid(&self) -> &'static [u8] {
        (self.uid)()
    }

    #[inline(always)]
    fn selected(&self) -> bool {
        self.selected
    }

    #[inline(always)]
    fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}

/// Largest unique id, in bytes.
pub const MAX_UID_LEN: usize = 16;

/// Handle `Select` (`[seed, prefix_len, prefix..]`): select this device
/// when its id starts with the `prefix_len`-bit prefix (bit `k` of an id
/// is `id[k / 8] >> (k % 8) & 1`), then wait its slot,
/// `wait_bytes(crc16(uid) >> (seed % 16) & 15)`, and reply `Ok` with its id;
/// otherwise deselect it. Returns whether to reply.
#[inline(always)]
pub(crate) fn select<A: Addressing, const N: usize>(
    a: &mut A,
    frame: &mut Frame<N>,
    wait_bytes: impl FnOnce(u8),
) -> bool {
    let uid = a.uid();
    // SAFETY: N >= 18 (Info) covers the 2 + 16 bytes used here.
    let data = unsafe { &mut frame.data.raw };
    let seed = data[0];
    // Bits of the prefix still to compare; any left after the id means a
    // prefix longer than the id, which matches nothing.
    let mut rem = data[1] as u32;
    let mut diff = 0;
    let mut i = 0;
    // One pass compares the prefix and copies the id over it for the
    // reply: byte i + 2 is read before byte i is written.
    // Bottom-tested: an id is never empty.
    loop {
        // SAFETY: i < uid.len() <= MAX_UID_LEN, and i + 2 < 2 + MAX_UID_LEN <= N.
        unsafe {
            let u = *uid.get_unchecked(i);
            let p = *data.get_unchecked(i + 2);
            *data.get_unchecked_mut(i) = u;
            let n = rem.min(8);
            rem -= n;
            diff |= (u ^ p) as u32 & (0xFF >> (8 - n));
        }
        i += 1;
        if i == uid.len() {
            break;
        }
    }
    let on = diff | rem == 0;
    a.set_selected(on);
    if !on {
        return false;
    }
    // The seed picks a window of the id's hash, not the CRC's initial
    // value: CRC is linear, so two ids' CRCs differ by the same bits under
    // every initial value, and a pair drawing one slot would draw it for
    // every seed.
    wait_bytes((crc16(CRC_INIT, uid) >> (seed & 15)) as u8 & 15);
    frame.len = uid.len() as u16;
    frame.status = Status::Ok;
    true
}
