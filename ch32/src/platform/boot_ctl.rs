//! CH32 boot control: run-mode intent, reset, hand-off to app.
//!
//! Intent is signalled across resets by one of three schemes, chosen by
//! chip capabilities and flash mode:
//!
//! - **reg**: flash `BOOT_MODE` register (system-flash, chips without `boot_pin`).
//! - **ram**: magic word in RAM (user-flash, all chips).
//! - **gpio**: RAM magic word + GPIO-driven BOOT0 circuit (system-flash,
//!   chips with `boot_pin`). The GPIO needs the caller-supplied
//!   `reset_delay_cycles` to let the external circuit (typically RC) settle.

use tinyboot_core::traits::BootCtl as TBBootCtl;
use tinyboot_core::traits::RunMode;

use crate::hal::pfic;

#[cfg(boot_req_gpio)]
use crate::hal::{Pin, gpio, rcc};

#[cfg(boot_req_ram)]
const BOOT_REQUEST_MAGIC: u32 = 0xB007_CAFE;

#[cfg(boot_req_ram)]
unsafe extern "C" {
    static mut __tb_boot_request: u32;
}

pub struct BootCtl {
    #[cfg(boot_req_gpio)]
    pin: Pin,
    #[cfg(boot_req_gpio)]
    active_high: bool,
    #[cfg(boot_req_gpio)]
    reset_delay_cycles: u32,
    #[cfg(not(feature = "system-flash"))]
    app_entry: u32,
}

impl BootCtl {
    #[cfg(boot_req_gpio)]
    #[inline(always)]
    pub fn new(pin: Pin, active_high: bool, reset_delay_cycles: u32) -> Self {
        rcc::enable_gpio(pin.port_index());
        gpio::configure(pin, gpio::PinMode::OUTPUT_PUSH_PULL);
        drive_boot_pin(pin, active_high, true);
        Self {
            pin,
            active_high,
            reset_delay_cycles,
        }
    }

    #[cfg(not(boot_req_gpio))]
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "system-flash"))]
            app_entry: {
                unsafe extern "C" {
                    static __tb_app_entry: u8;
                }
                unsafe { &__tb_app_entry as *const u8 as u32 }
            },
        }
    }
}

impl TBBootCtl for BootCtl {
    #[inline(always)]
    fn run_mode(&self) -> RunMode {
        #[cfg(boot_req_reg)]
        let service = crate::hal::flash::boot_mode();
        #[cfg(boot_req_ram)]
        let service =
            unsafe { core::ptr::read_volatile(&raw const __tb_boot_request) } == BOOT_REQUEST_MAGIC;
        if service {
            RunMode::Service
        } else {
            RunMode::HandOff
        }
    }

    #[inline(always)]
    fn set_run_mode(&mut self, mode: RunMode) {
        let service = mode == RunMode::Service;

        #[cfg(boot_req_reg)]
        crate::hal::flash::set_boot_mode(service);

        #[cfg(boot_req_ram)]
        {
            let val = if service { BOOT_REQUEST_MAGIC } else { 0 };
            unsafe { core::ptr::write_volatile(&raw mut __tb_boot_request, val) };
        }

        #[cfg(boot_req_gpio)]
        {
            drive_boot_pin(self.pin, self.active_high, service);
            crate::hal::delay_cycles(self.reset_delay_cycles);
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> ! {
        pfic::software_reset()
    }

    #[inline(always)]
    fn hand_off(&mut self) -> ! {
        #[cfg(not(feature = "system-flash"))]
        {
            crate::hal::rcc::reset_apb2();
            pfic::jump(self.app_entry)
        }
        #[cfg(feature = "system-flash")]
        {
            self.set_run_mode(RunMode::HandOff);
            pfic::software_reset()
        }
    }
}

#[cfg(boot_req_gpio)]
#[inline(always)]
fn drive_boot_pin(pin: Pin, active_high: bool, system_flash: bool) {
    let level = if active_high == system_flash {
        gpio::Level::High
    } else {
        gpio::Level::Low
    };
    gpio::set_level(pin, level);
}
