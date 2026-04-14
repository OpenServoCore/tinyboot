use super::{BootMode, BootState};

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

    /// Unlock flash for erase/write. Called once before entering the protocol loop.
    fn unlock(&mut self);
}

/// Trait for system boot control.
pub trait BootCtl {
    /// Returns true if the bootloader was explicitly requested (e.g. via boot mode register).
    fn is_boot_requested(&self) -> bool;

    /// Reset the system into the specified boot mode.
    fn system_reset(&mut self, mode: BootMode) -> !;
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

/// Concrete platform holding all boot-time peripherals.
///
/// Constructed by the board-specific crate and passed to [`Core::new`](crate::Core::new).
pub struct Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    /// UART / RS-485 transport.
    pub transport: T,
    /// Flash storage for reading and writing firmware.
    pub storage: S,
    /// Persistent boot metadata (state, trials, checksum).
    pub boot_meta: B,
    /// Boot control (reset, boot mode selection).
    pub ctl: C,
}

impl<T, S, B, C> Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    /// Assemble a platform from its components.
    #[inline(always)]
    pub fn new(transport: T, storage: S, boot_meta: B, ctl: C) -> Self {
        Self {
            transport,
            storage,
            boot_meta,
            ctl,
        }
    }
}
