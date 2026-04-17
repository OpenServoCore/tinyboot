//! Boot-source latch: selects which image the factory ROM dispatches to next.
//!
//! Only compiled when `feature = "system-flash"`. Exactly one variant:
//! - [`mode`]: BOOT_MODE register           (V003 + system-flash).
//! - [`gpio`]: GPIO + external BOOT0 circuit (V103 + system-flash).

/// Which image the factory ROM should dispatch to on the next reset.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BootSrc {
    UserFlash,
    SystemFlash,
}

core::cfg_select! {
    boot_src_mode => {
        mod mode;
        pub type Active = mode::ModeBootSrcCtl;
    }
    boot_src_gpio => {
        mod gpio;
        pub type Active = gpio::GpioBootSrcCtl;
    }
}
