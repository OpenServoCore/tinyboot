//! Boot request signaling and boot mode selection.
//!
//! Three schemes selected automatically by chip capabilities and flash mode:
//!
//! - **reg**: `BOOT_MODE` register (system-flash, chips without `boot_pin`)
//! - **ram**: RAM magic word (user-flash, all chips)
//! - **gpio**: RAM magic word + GPIO pin (system-flash, chips with `boot_pin`)
//!
//! Scheme cfgs (`boot_req_reg`, `boot_req_ram`, `boot_req_gpio`) are set by
//! the build script. `ram` and `gpio` are both active for system-flash + boot_pin.

// ── Configuration ──────────────────────────────────────────────────────

/// Boot control hardware configuration.
///
/// For the gpio scheme (system-flash on chips with `boot_pin`), this
/// configures the GPIO pin connected to the BOOT0 control circuit.
/// For all other schemes, no hardware configuration is needed.
#[cfg(boot_req_gpio)]
#[derive(Copy, Clone)]
pub struct Config {
    /// GPIO pin connected to the BOOT0 control circuit.
    pub pin: crate::Pin,
    /// `true` if driving HIGH selects system flash (default for RC circuit).
    pub active_high: bool,
}

/// Boot control hardware configuration (no-op for reg and ram schemes).
#[cfg(not(boot_req_gpio))]
#[derive(Copy, Clone, Default)]
pub struct Config;

// ── Public API ─────────────────────────────────────────────────────────

/// Initialize boot control hardware. Call once at startup.
///
/// For the gpio scheme, configures the BOOT_CTL pin as push-pull output
/// and defaults it to the system-flash direction. For other schemes this
/// is a no-op.
pub fn init(_config: &Config) {
    #[cfg(boot_req_gpio)]
    {
        use crate::gpio::{self, PinMode};
        crate::rcc::enable_gpio(_config.pin.port_index());
        gpio::configure(_config.pin, PinMode::OUTPUT_PUSH_PULL);
        drive_boot_pin(_config.pin, _config.active_high, true);
    }
}

/// Check whether a boot-to-bootloader request is pending.
pub fn is_boot_requested() -> bool {
    #[cfg(boot_req_reg)]
    return crate::flash::boot_mode();

    #[cfg(boot_req_ram)]
    return unsafe { core::ptr::read_volatile(&raw const __tb_boot_request) == BOOT_REQUEST_MAGIC };
}

/// Signal (or clear) boot intent for the next reset.
pub fn set_boot_request(_config: &Config, request: bool) {
    #[cfg(boot_req_reg)]
    crate::flash::set_boot_mode(request);

    #[cfg(boot_req_ram)]
    {
        let val = if request { BOOT_REQUEST_MAGIC } else { 0 };
        unsafe { core::ptr::write_volatile(&raw mut __tb_boot_request, val) };
    }

    #[cfg(boot_req_gpio)]
    drive_boot_pin(_config.pin, _config.active_high, request);
}

// ── Private ────────────────────────────────────────────────────────────

#[cfg(boot_req_ram)]
const BOOT_REQUEST_MAGIC: u32 = 0xB007_CAFE;

#[cfg(boot_req_ram)]
unsafe extern "C" {
    static mut __tb_boot_request: u32;
}

#[cfg(boot_req_gpio)]
fn drive_boot_pin(pin: crate::Pin, active_high: bool, system_flash: bool) {
    use crate::gpio::{self, Level};
    let level = if active_high == system_flash {
        Level::High
    } else {
        Level::Low
    };
    gpio::set_level(pin, level);
}
