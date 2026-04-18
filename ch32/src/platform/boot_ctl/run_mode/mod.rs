//! Run-mode persisted across reset. Variants (one per build):
//! - `mode`: flash BOOT_MODE register (V003 + system-flash).
//! - `ram`: magic word in RAM (everything else).

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
