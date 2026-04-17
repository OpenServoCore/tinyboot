//! CH32 boot control: run-mode intent, boot-source latch, hand-off to app.
//!
//! Composes three orthogonal concerns:
//! - [`run_mode`]: how run-mode intent survives a reset (mode/ram).
//! - [`boot_src`]: which image the factory ROM dispatches to next reset
//!   (mode/gpio). Only present under `feature = "system-flash"`.
//! - [`hand_off`]: how control transfers to the app (software reset vs jump).

use tinyboot_core::traits::{BootCtl as TBBootCtl, RunMode};

use crate::hal::pfic;

#[cfg(feature = "system-flash")]
mod boot_src;
mod hand_off;
mod run_mode;

#[cfg(boot_src_gpio)]
use crate::hal::Pin;
#[cfg(feature = "system-flash")]
use boot_src::BootSrc;

pub struct BootCtl {
    run_mode: run_mode::Active,
    #[cfg(feature = "system-flash")]
    boot_src: boot_src::Active,
    hand_off: hand_off::Active,
}

impl BootCtl {
    core::cfg_select! {
        boot_src_gpio => {
            #[inline(always)]
            pub fn new(pin: Pin, active_high: bool, reset_delay_cycles: u32) -> Self {
                Self {
                    run_mode: run_mode::Active::new(),
                    boot_src: boot_src::Active::new(pin, active_high, reset_delay_cycles),
                    hand_off: hand_off::Active::new(),
                }
            }
        }
        _ => {
            #[inline(always)]
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self {
                    run_mode: run_mode::Active::new(),
                    #[cfg(feature = "system-flash")]
                    boot_src: boot_src::Active::new(),
                    hand_off: hand_off::Active::new(),
                }
            }
        }
    }
}

impl TBBootCtl for BootCtl {
    #[inline(always)]
    fn run_mode(&self) -> RunMode {
        self.run_mode.read()
    }

    #[inline(always)]
    fn set_run_mode(&mut self, mode: RunMode) {
        self.run_mode.write(mode);
        #[cfg(feature = "system-flash")]
        {
            let src = match mode {
                RunMode::Service => BootSrc::SystemFlash,
                RunMode::HandOff => BootSrc::UserFlash,
            };
            self.boot_src.set(src);
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> ! {
        pfic::software_reset()
    }

    #[inline(always)]
    fn hand_off(&mut self) -> ! {
        // Persist HandOff and latch the ROM toward user flash before executing,
        // so a power cycle after this path still boots the app.
        self.run_mode.write(RunMode::HandOff);
        #[cfg(feature = "system-flash")]
        self.boot_src.set(BootSrc::UserFlash);
        self.hand_off.execute()
    }
}
