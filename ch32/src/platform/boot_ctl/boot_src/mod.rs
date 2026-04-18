//! Boot-source latch (system-flash only). Variants:
//! - `mode`: flash BOOT_MODE register (V003).
//! - `gpio`: GPIO driving external BOOT0 circuit (V103).

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
