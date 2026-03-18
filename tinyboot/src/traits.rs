/// Trait for firmware transfer protocol.
pub trait Transport: embedded_io::Read + embedded_io::Write {}
impl<T> Transport for T where T: embedded_io::Read + embedded_io::Write {}

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

/// Trait for system boot control.
pub trait BootCtl {
    /// Returns true if the bootloader was explicitly requested (e.g. via boot mode register).
    fn is_boot_requested(&self) -> bool;

    /// Clear the boot request flag so the next reset boots the app.
    fn clear_boot_request(&mut self);

    /// Reset the system.
    fn system_reset(&mut self) -> !;

    /// Clear boot request and jump/reset into the app.
    fn boot_app(&mut self) -> !;
}

/// Current stage in the firmware update lifecycle.
///
/// Each state is a contiguous run of 1-bits from bit 0.
/// Advancing clears the MSB: `next = state & (state >> 1)`.
///
/// ```text
/// 0xFFFF  Idle        (16 ones)
/// 0x7FFF  Updating    (15 ones)
/// 0x3FFF  Validating  (14 ones)
/// 0x1FFF  Confirmed   (13 ones)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u16)]
pub enum BootState {
    /// No update in progress. Normal app boot. Erased flash default.
    Idle = 0xFFFF,
    /// Firmware transfer in progress.
    Updating = 0x7FFF,
    /// New firmware written, trial booting the app.
    Validating = 0x3FFF,
    /// App confirmed successful boot.
    Confirmed = 0x1FFF,
    /// Stored value doesn't match any valid variant.
    Corrupt = 0x0000,
}

impl BootState {
    pub fn from_u16(v: u16) -> Self {
        match v {
            0xFFFF => BootState::Idle,
            0x7FFF => BootState::Updating,
            0x3FFF => BootState::Validating,
            0x1FFF => BootState::Confirmed,
            _ => BootState::Corrupt,
        }
    }
}

/// Persistent boot metadata.
///
/// Stored in flash at a known address. Fields are laid out so that
/// forward state transitions and trial consumption only require 1→0
/// bit writes (no erase). A full erase + write is only needed to
/// return to a blank state.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct BootMeta {
    /// Current boot lifecycle state.
    pub state: u16,
    /// Trial boot counter. Each consumed trial clears one bit (1→0).
    /// 0xFFFF = 16 remaining, ..., 0x0000 = exhausted.
    pub trials: u16,
    /// Checksum of the application firmware image.
    pub app_checksum: u32,
    /// Size of the application firmware image in bytes.
    pub app_size: u32,
}

impl BootMeta {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Number of trial boots remaining before exhausted.
    pub fn trials_remaining(&self) -> u8 {
        self.trials.count_ones() as u8
    }

    /// Decode the state field.
    pub fn boot_state(&self) -> BootState {
        BootState::from_u16(self.state)
    }
}

/// Persistent boot metadata storage.
///
/// Provides read access to the full `BootMeta` struct and forward-only
/// state transitions (1→0 writes). No explicit write/reset is needed:
/// erased storage (all 0xFF) naturally represents the default state
/// (Idle, full trials). The host writes the meta struct as part of
/// the normal firmware transfer.
pub trait BootMetaStore {
    type Error: core::fmt::Debug;

    /// Read the current boot metadata.
    fn read(&self) -> BootMeta;

    /// Advance the boot state forward by one step.
    /// Returns the new state on success.
    /// Errors if the state is `Confirmed` or `Corrupt`.
    fn advance(&mut self) -> Result<BootState, Self::Error>;

    /// Consume one trial boot (clears one bit in the trials field).
    /// Errors if trials are already exhausted.
    fn consume_trial(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// App-side boot client interface.
///
/// Provides the two operations an application needs from the bootloader:
/// confirming a successful trial boot, and requesting bootloader entry
/// for a firmware update.
pub trait BootClient {
    /// Confirm a successful boot.
    ///
    /// If the boot state is `Validating`, advances it to `Confirmed`.
    /// Otherwise does nothing (already confirmed or no update in progress).
    fn confirm(&mut self);

    /// Request bootloader entry for a firmware update.
    ///
    /// Writes the boot request flag and performs a soft reset.
    /// This function does not return.
    fn request_update(&mut self) -> !;
}

pub struct Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    pub transport: T,
    pub storage: S,
    pub boot_meta: B,
    pub ctl: C,
}

impl<T, S, B, C> Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    pub fn new(transport: T, storage: S, boot_meta: B, ctl: C) -> Self {
        Self {
            transport,
            storage,
            boot_meta,
            ctl,
        }
    }
}
