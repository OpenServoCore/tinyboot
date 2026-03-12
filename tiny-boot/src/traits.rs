/// Trait for firmware transfer protocol.
pub trait Transport: embedded_io::Read + embedded_io::Write {}
impl<T> Transport for T where T: embedded_io::Read + embedded_io::Write {}

/// Trait for reading and writing firmware to persistent storage.
pub trait Storage:
    embedded_storage::nor_flash::NorFlash + embedded_storage::nor_flash::ReadNorFlash
{
}
impl<T> Storage for T where
    T: embedded_storage::nor_flash::NorFlash + embedded_storage::nor_flash::ReadNorFlash
{
}

/// Trait for system / app interactions.
pub trait BootCtl {
    /// Jump to the app entry point.
    fn jump_to_app(&self) -> !;

    /// Reset the system after flash operations.
    fn system_reset(&mut self) -> !;
}

/// Current stage in the firmware update lifecycle.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BootState {
    /// No update in progress. Normal app boot.
    Idle,
    /// Firmware transfer in progress.
    Updating,
    /// New firmware written, trial booting the app.
    Validating,
    /// App confirmed successful boot.
    Confirmed,
}

/// Persistent boot state storage.
///
/// Models a state machine for firmware update lifecycle. Implementations
/// may be backed by option bytes, a flash page, EEPROM, backup registers,
/// or any persistent storage.
///
/// State transitions:
///
///   TrialBoot:
///     Idle → Updating → Validating → Confirmed → Idle
///     The Validating state uses a trial counter — if the app fails to
///     confirm within N boots, we stay in bootloader mode, so that the
///     user can attempt to reflash the app.
///
///   Relaxed:
///     Idle → Updating → Confirmed → Idle
///     Skips validation — flash and go.
///
pub trait BootStateStore {
    type Error;

    /// Whether the app has requested to enter the bootloader.
    fn boot_requested(&mut self) -> Result<bool, Self::Error>;

    /// Read the current boot state.
    fn state(&mut self) -> Result<BootState, Self::Error>;

    /// Advance to the next state.
    fn transition(&mut self) -> Result<BootState, Self::Error>;

    /// Decrement the trial boot counter by one.
    fn increment_trial(&mut self) -> Result<(), Self::Error>;

    /// Number of trial boots remaining before the bootloader should roll back.
    fn trials_remaining(&mut self) -> Result<u8, Self::Error>;
}

pub struct Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootStateStore,
    C: BootCtl,
{
    pub transport: T,
    pub storage: S,
    pub boot_state: B,
    pub ctl: C,
}

impl<T, S, B, C> Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootStateStore,
    C: BootCtl,
{
    pub fn new(transport: T, storage: S, boot_state: B, ctl: C) -> Self {
        Self {
            transport,
            storage,
            boot_state,
            ctl,
        }
    }
}
