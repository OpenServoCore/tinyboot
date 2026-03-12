// Magic number. Used to verify the app's integrity before execution.
pub const APP_MAGIC: u32 = 0xC0FF_EEEE;

pub trait Transport: embedded_io::Read + embedded_io::Write {}
impl<T> Transport for T where T: embedded_io::Read + embedded_io::Write {}

pub trait Flash: embedded_storage::nor_flash::NorFlash + embedded_storage::nor_flash::ReadNorFlash {}
impl<T> Flash for T where T: embedded_storage::nor_flash::NorFlash + embedded_storage::nor_flash::ReadNorFlash {}

/// Trait for system / app interactions.
pub trait Abi {
    /// Read the app magic number embedded in the app.
    /// This is used to verify the app's integrity before execution.
    /// implement this as MMIO read from app flash (whereever you embedded it)
    fn app_magic(&self) -> u32;

    /// Get the flash region where the app is located.
    /// implement this using constants in memory.x file and use extern "C"
    /// to retrieve these values.
    fn app_flash_region(&self) -> (u32, u32);

    /// Jump to the app entry point.
    fn jump_to_app(&self) -> !;

    /// Reset the system after flash operations.
    fn system_reset(&mut self) -> !;
}

/// Boot Metadata registers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegistryKey {
    BootRequest,
    // more in the future
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootRequest {
    Application = 0x00,
    Bootloader = 0x01,
    Invalid = 0xFF,
}

/// Trait for reading and writing boot metadata.
/// For CH32, use Option Byte DATA0/DATA1
/// For other platforms, one can use backup registers or a flash page.
pub trait Registry {
    type Error;

    fn read(&mut self, key: RegistryKey) -> Result<u8, Self::Error>;
    fn write(&mut self, key: RegistryKey, value: u8) -> Result<(), Self::Error>;

    fn read_boot_request(&mut self) -> Result<BootRequest, Self::Error> {
        self.read(RegistryKey::BootRequest).map(|v| match v {
            0x00 => BootRequest::Application,
            0x01 => BootRequest::Bootloader,
            _ => BootRequest::Invalid,
        })
    }

    fn write_boot_request(&mut self, br: BootRequest) -> Result<(), Self::Error> {
        self.write(RegistryKey::BootRequest, br as u8)
    }
}

pub struct Hal<T, F, R, A>
where
    T: Transport,
    F: Flash,
    R: Registry,
    A: Abi,
{
    pub transport: T,
    pub flash: F,
    pub reg: R,
    pub abi: A,
}

impl<T, F, R, A> Hal<T, F, R, A>
where
    T: Transport,
    F: Flash,
    R: Registry,
    A: Abi,
{
    pub fn new(transport: T, flash: F, reg: R, abi: A) -> Self {
        Self {
            transport,
            flash,
            reg,
            abi,
        }
    }
}
