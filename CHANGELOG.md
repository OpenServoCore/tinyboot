# Changelog

## [Unreleased]

### Changed

- **Breaking:** merged `tinyboot-ch32-hal`, `tinyboot-ch32-boot`, and `tinyboot-ch32-app` into a single `tinyboot-ch32` crate. Use `tinyboot_ch32::boot` in bootloader binaries and `tinyboot_ch32::app` in applications; `tinyboot_ch32::hal` is the HAL module. Chip features (`ch32v003f4p6`, `ch32v103c8t6`, …) and `system-flash` move to the unified crate.
- **Breaking:** extracted the minimal bootloader startup into a new `tinyboot-ch32-rt` crate. Bootloader binaries now depend on `tinyboot-ch32-rt` (e.g. `use tinyboot_ch32_rt as _;`); apps keep using `qingke-rt`.
- **Breaking:** `tinyboot_core::traits` flattened into a single module. `tinyboot_core::traits::app::BootClient` is gone; app-side behaviour now lives in `tinyboot_core::app::App`. `Platform` moved to `tinyboot_core::Platform` (re-export kept at crate root).
- **Breaking:** replaced `BootMode` with `RunMode { HandOff, Service }` to separate "what to do after reset" from "which image the ROM dispatches next boot".
- **Breaking:** reshaped `BootCtl`. Was `is_boot_requested()` + `system_reset(BootMode)`; now `run_mode()` / `set_run_mode(RunMode)` for persisted intent, `reset()` for software reset, and `hand_off()` for transferring control to the app.
- **Breaking:** removed `Storage::unlock()`. Flash unlock/lock is now scoped to each operation inside the CH32 HAL, so storage is self-locking.
- **Breaking:** CH32 `BootCtl::new` on V103 + `system-flash` takes `(Pin, Level, u32)` for the external BOOT0 circuit — the GPIO pin, the level that selects the system-flash bootloader, and a reset delay (CPU cycles) that lets the RC/flip-flop settle before reset. Other combinations stay unit-arg.
- CH32 `platform::boot_ctl` restructured into orthogonal `run_mode`, `boot_src`, and `hand_off` submodules dispatched via `core::cfg_select!` based on chip and flash mode.
- Run-mode persistence now uses variant-specific backends: `BOOT_MODE` register (V003 + system-flash) or a RAM magic word elsewhere. The magic-word linker script (`tb-run-mode.x`) ships from `tinyboot-ch32` and is linked by both bootloader and app.
- Converted non-HAL `bool` parameters to enums for clarity (`Duplex`, `Level`, `Pull`, `RunMode`, `BootSrc`).
- Renamed `boot_state.rs` → `boot_meta_store.rs` in the CH32 platform for consistency with the `BootMetaStore` trait.
- Aliased crate names removed — crates are now referenced by their package name everywhere.

### Added

- `tinyboot-ch32-rt` crate: tiny `_start` + `link.x` for bootloader binaries that can't afford full `qingke-rt` (critical on system-flash targets).
- README for `tinyboot-ch32` and `tinyboot-ch32-rt` covering module layout, usage, and linker-script conventions.

## [0.3.0] - 2026-04-15

### Added

- **CH32V103 support** — full bootloader and app support for CH32V103 (Qingke V3A core), including system-flash and user-flash modes with GPIO-controlled boot mode selection
- **Decoupled protocol frame size from flash page size** — ring buffer accumulates writes and flushes full pages, with new Flush command and fast write support
- **CLI retry on CRC mismatch** — automatic retry from page boundary on corrupted response frames

### Changed

- **Breaking:** reorganized into multi-workspace structure (lib/, ch32/, cli/, examples/)
- **Breaking:** `tinyboot` crate renamed to `tinyboot-core`; CLI crate renamed from `tinyboot-cli` to `tinyboot`
- **Breaking:** boot metadata moved from option bytes to last page of user flash
- Switched to OpenServoCore fork of qingke/qingke-rt — fixes mtvec, adds V3A support; removed `fix_mtvec!()` workaround
- Removed defmt and `tinyboot-macros` from bootloader
- Multi-chip CI coverage for all CH32V003 and CH32V103 variants

### Fixed

- Protocol write alignment check only on first write
- UB in boot metadata reads

## [0.2.1] - 2026-03-25

### Fixed

- **UB in boot metadata reads** — fixed memory alignment issue by using `u32` buffer and casting back to `u8` array
- **App version display** — fixed `app_version` read and app boot for user-flash example
- **mtvec for apps behind bootloader** — `qingke-rt` hardcodes `mtvec = 0x0`, breaking interrupts in apps loaded at non-zero addresses; added `fix_mtvec!` macro to `tinyboot-ch32-app` that wraps `_setup_interrupts` via linker `--wrap` to rewrite `mtvec` to the actual vector table base
- **Peripheral cleanup before app jump** — properly reset APB2 peripherals (`rcc::reset_apb2`) before jumping to app, preventing stale peripheral state from leaking into the application
- **defmt panics on app→bootloader reset** — split bootloader runtime into `v2.S` (minimal, no .data/.bss init) and `v2_full.S` (full init for defmt); the `defmt` feature selects the appropriate startup

### Added

- **CLI logging** — `env_logger` support; set `RUST_LOG=debug` for protocol-level diagnostics

### Optimized

- ~180 bytes saved in system-flash bootloader via aggressive inlining, CRC/payload merge, batched RCC enable, custom `read_exact`/`write_all` overrides, and boot version cleanup
- All CH32V003 chip variants added with CI coverage

## [0.2.0] - 2026-03-20

### Changed

- **Breaking:** Verify command now carries `app_size` in the addr field
- **Breaking:** `BootMetaStore` trait: `trials_remaining()` replaced by `has_trials() -> bool`; `refresh()` takes an additional `app_size` parameter
- **Breaking:** `BootMetaStore::new()` replaced by `Default` impl (`BootMetaStore::default()`)
- **Breaking:** `BootCtl::system_reset()` takes `BootMode` enum (`App` / `Bootloader`) instead of `bool`
- CRC16 validation now covers only actual firmware bytes, not the entire flash region
- CLI only writes actual firmware data — no more 0xFF padding to fill the region
- App version read from end of binary (`flash[app_size-2..app_size]`) instead of end of flash region
- Linker script places `.tinyboot_version` after all other flash content (end of binary) instead of at end of flash region
- OB metadata expanded from 8 to 16 bytes (added app_size u32 field)
- System flash memory.x corrected to LENGTH=1920 (actual system flash size)

### Added

- `iwdg::feed()` in HAL — feeds the independent watchdog timer before OB erase in app-side `confirm()` to prevent watchdog reset during the critical OB erase+rewrite window
- `BootMode` enum (`App` / `Bootloader`) — replaces bare `bool` in boot control APIs
- `has_trials() -> bool` on `BootMetaStore` trait — simpler and avoids software popcount on targets without hardware support

### Optimized

- Startup assembly stripped to 20 bytes (from 88) — removed .data copy loop, .bss zero loop, and alignment padding since the bootloader uses no mutable statics
- Flash time reduced proportionally to firmware size (e.g. 5KB app on 16KB chip: ~8s vs full-region flash)
- CRC verification faster — only covers firmware bytes

## [0.1.0] - 2026-03-20

Initial release.
