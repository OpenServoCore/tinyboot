/// App-side boot client interface.
pub mod app;
/// Boot-side platform traits.
pub mod boot;

/// Boot target after a system reset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootMode {
    /// Boot the application.
    App,
    /// Enter the bootloader.
    Bootloader,
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
