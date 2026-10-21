//! Platform abstraction traits.

/// Tinyboot's post-reset action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunMode {
    /// Boot the application.
    HandOff,
    /// Stay in the bootloader, service protocol commands.
    Service,
}

/// Firmware-update lifecycle stage. Encoded as contiguous 1-bit runs so
/// advancing is a 1→0 bit-clear: `next = state & (state >> 1)`.
///
/// `0xFF` Idle → `0x7F` Updating → `0x3F` Validating.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BootState {
    /// No update in progress (erased flash default).
    Idle = 0xFF,
    /// Firmware transfer in progress.
    Updating = 0x7F,
    /// New firmware written, trial booting.
    Validating = 0x3F,
}

impl BootState {
    /// Parse a raw byte. Unrecognised values decay to [`Idle`](BootState::Idle).
    pub fn from_u8(v: u8) -> Self {
        match v {
            0xFF | 0x7F | 0x3F => unsafe { core::mem::transmute::<u8, BootState>(v) },
            _ => BootState::Idle,
        }
    }
}

/// Firmware transfer transport.
pub trait Transport: embedded_io::Read + embedded_io::Write {
    /// Wait about `n` byte-times (10 bit-times each) at the current baud.
    /// An addressable device waits a draw of 0 to 15 before its `Select`
    /// reply, so devices running the same code do not answer in unison.
    /// The default returns at once; give a shared bus a real wait (it may
    /// overshoot).
    fn wait_bytes(&mut self, n: u8) {
        let _ = n;
    }
}

/// App-region flash. [`as_slice`](Storage::as_slice) gives zero-copy reads
/// (flash is memory-mapped on supported chips).
pub trait Storage:
    embedded_storage::nor_flash::NorFlash + embedded_storage::nor_flash::ReadNorFlash
{
    /// Zero-copy read access to the app region.
    fn as_slice(&self) -> &[u8];

    /// The app region's first word is programmed. Without a stored CRC
    /// this is the only sign of an app: an erased chip reads all 0xFF.
    /// A platform that knows its region holds a word may skip the length
    /// check.
    #[inline(always)]
    fn app_present(&self) -> bool {
        let data = self.as_slice();
        // SAFETY: flash is word-aligned and the length is checked first.
        data.len() >= 4
            && unsafe { core::ptr::read_volatile(data.as_ptr() as *const u32) } != 0xFFFF_FFFF
    }
}

/// Boot control primitives.
pub trait BootCtl {
    /// Read persisted run mode.
    fn run_mode(&self) -> RunMode;

    /// Set run mode for the next reset.
    fn set_run_mode(&mut self, mode: RunMode);

    /// Software reset.
    fn reset(&mut self) -> !;

    /// Transfer control to the application.
    fn hand_off(&mut self) -> !;
}

/// Persistent boot metadata.
pub trait BootMetaStore {
    /// Error returned by mutating methods.
    type Error: core::fmt::Debug;

    /// Current lifecycle state.
    fn boot_state(&self) -> BootState;

    /// Any trials left?
    fn has_trials(&self) -> bool;

    /// Stored app CRC16.
    fn app_checksum(&self) -> u16;

    /// Stored app size in bytes.
    fn app_size(&self) -> u32;

    /// Stored bootloader baud index (a `BaudRate` discriminant; `0xFF` =
    /// the bootloader's build default).
    fn baud(&self) -> u8;

    /// Erase and rewrite meta with a new baud index; everything else kept.
    fn set_baud(&mut self, baud: u8) -> Result<(), Self::Error>;

    /// Advance state (1→0 bit clear).
    fn advance(&mut self) -> Result<BootState, Self::Error>;

    /// Clear one bit of the trials counter.
    fn consume_trial(&mut self) -> Result<(), Self::Error>;

    /// Erase and rewrite meta. Trials reset to the erased default; the
    /// baud index is kept.
    fn refresh(
        &mut self,
        checksum: u16,
        state: BootState,
        app_size: u32,
    ) -> Result<(), Self::Error>;
}
