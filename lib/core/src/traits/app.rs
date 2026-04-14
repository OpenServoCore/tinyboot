/// App-side boot client interface.
///
/// Provides the operations an application needs from the bootloader:
/// confirming a successful trial boot, requesting bootloader entry
/// for a firmware update, and performing a system reset.
pub trait BootClient {
    /// Confirm a successful boot.
    ///
    /// If the boot state is `Validating`, refreshes metadata back to Idle.
    /// Otherwise does nothing (already confirmed or no update in progress).
    fn confirm(&mut self);

    /// Set the boot request flag so the next reset enters the bootloader.
    fn request_update(&mut self);

    /// Reset the system. This function does not return.
    fn system_reset(&mut self) -> !;
}
