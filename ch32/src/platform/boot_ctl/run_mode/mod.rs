//! Run-mode intent persisted across reset.
//!
//! Exactly one variant is selected per build by `build.rs` cfgs:
//! - [`mode`]: flash BOOT_MODE register (V003 + system-flash).
//! - [`ram`]:  magic word in RAM (everything else).
//!
//! Future variants (unimplemented): `mscratch` (CSR), `bkp` (backup domain).

core::cfg_select! {
    run_mode_mode => {
        mod mode;
        pub type Active = mode::ModeRunModeCtl;
    }
    run_mode_ram => {
        mod ram;
        pub type Active = ram::RamRunModeCtl;
    }
}
