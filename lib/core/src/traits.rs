//! Platform abstraction traits.

/// What tinyboot does after reset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunMode {
    /// Hand off to the application.
    HandOff,
    /// Stay in the bootloader and service commands.
    Service,
}

/// Current stage in the firmware update lifecycle.
///
/// Each state is a contiguous run of 1-bits from bit 0.
/// Advancing clears the MSB: `next = state & (state >> 1)`.
///
/// ```text
/// 0xFF  Idle        (8 ones)
/// 0x7F  Updating    (7 ones)
/// 0x3F  Validating  (6 ones)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BootState {
    /// No update in progress. Normal app boot. Erased flash default.
    Idle = 0xFF,
    /// Firmware transfer in progress.
    Updating = 0x7F,
    /// New firmware written, trial booting the app.
    Validating = 0x3F,
}

impl BootState {
    /// Parse a raw byte into a [`BootState`]. Unrecognised values default to [`Idle`](BootState::Idle).
    pub fn from_u8(v: u8) -> Self {
        match v {
            0xFF | 0x7F | 0x3F => unsafe { core::mem::transmute::<u8, BootState>(v) },
            _ => BootState::Idle,
        }
    }
}

/// Trait for firmware transfer protocol.
pub trait Transport: embedded_io::Read + embedded_io::Write {}

/// Trait for reading and writing firmware to persistent storage.
///
/// Flash is memory-mapped, so [`as_slice`](Storage::as_slice) provides
/// zero-copy read access to the app region.
pub trait Storage:
    embedded_storage::nor_flash::NorFlash + embedded_storage::nor_flash::ReadNorFlash
{
    /// Direct read access to the app region (zero-copy).
    fn as_slice(&self) -> &[u8];
}

/// Boot control primitives exposed to the core state machine.
pub trait BootCtl {
    /// Read the persistent run-mode intent.
    fn run_mode(&self) -> RunMode;

    /// Write the run-mode intent for the next reset.
    fn set_run_mode(&mut self, mode: RunMode);

    /// Software reset.
    fn reset(&mut self) -> !;

    /// Transfer control to the application.
    fn hand_off(&mut self) -> !;
}

/// Persistent boot metadata storage.
pub trait BootMetaStore {
    /// Error type for metadata operations.
    type Error: core::fmt::Debug;

    /// Current boot lifecycle state.
    fn boot_state(&self) -> BootState;

    /// Returns true if any trial boots remain.
    fn has_trials(&self) -> bool;

    /// Stored CRC16 of the application firmware.
    fn app_checksum(&self) -> u16;

    /// Stored application size in bytes.
    fn app_size(&self) -> u32;

    /// Step state down by one (1→0 bit clear).
    fn advance(&mut self) -> Result<BootState, Self::Error>;

    /// Consume one trial boot (clears one bit in the trials field).
    fn consume_trial(&mut self) -> Result<(), Self::Error>;

    /// Erase meta and rewrite with given checksum, state, and app_size.
    /// Trials return to erased default (full).
    fn refresh(
        &mut self,
        checksum: u16,
        state: BootState,
        app_size: u32,
    ) -> Result<(), Self::Error>;
}
