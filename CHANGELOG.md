# Changelog

Releases up through `v0.4.0` were workspace-wide and tagged `vX.Y.Z`. From `0.4.1` onwards crates ship independently and headers/tags use the form `<crate> X.Y.Z` / `<crate>-vX.Y.Z`.

## [Unreleased]

## [tinyboot-ch32-rt 0.4.1] - 2026-05-08

### Fixed

- Trim `link.x` to drop unused vector-table / exception / trap KEEPs the bootloader never produces. Boot binaries are now tolerant of downstream PACs (e.g. `ch32-metapac` with the `rt` feature) emitting `__EXTERNAL_INTERRUPTS` under cargo feature unification — `--gc-sections` drops the unreferenced static. Bootloader flash size is unchanged.

## [tinyboot-ch32 0.4.1] - 2026-05-07

### Added

- Re-export `ch32-metapac` as `tinyboot_ch32::pac` so bootloader binaries can do register-level customization (e.g. clock or flash-latency setup before `boot::run`) without a separate metapac dep.
- `BaudRate` enum extended up to 3 Mbps (`B500000`, `B750000`, `B1000000`, `B1500000`, `B2000000`, `B3000000`).

## [tinyboot 0.4.1] - 2026-05-07

### Changed

- On a single-wire bus the host hears its own request frame back; the client now skips frames whose status is `Request` so single-wire / DXL TTL setups work without extra flags.

## [v0.4.0] - 2026-04-19

### Added

- **CH32V00x support** — full bootloader and app support for CH32V002/V004/V005/V006/V007 (system flash at `0x1FFF0000`, 3KB + 256B).
- `tinyboot-ch32-rt` crate: minimal `_start` + `link.x` for bootloader binaries that can't afford full `qingke-rt`.
- V103 split BOOT/BOOT2 system-flash regions: UART transport placed in `.text2` (second region) so all features fit.
- `tx_en` support in the CH32V00x example app for RS-485 / DXL TTL loopback.

### Changed

- **Breaking:** protocol address reduced to 24 bits; addr byte 3 is now a per-command `Flags` byte. `WriteFlags::FLUSH` replaces the standalone `Cmd::Flush`; `ResetFlags::BOOTLOADER` replaces `addr=1` signaling on Reset.
- **Breaking:** `Frame::addr: u32` split into `addr_lo: u16` + `addr_hi: u8` + `flags: Flags` union; use `Frame::addr()` / `Frame::set_addr()` accessors.
- **Breaking:** merged `tinyboot-ch32-hal`, `tinyboot-ch32-boot`, and `tinyboot-ch32-app` into a single `tinyboot-ch32` crate (`::boot`, `::app`, `::hal` modules).
- **Breaking:** replaced `BootMode` with `RunMode { HandOff, Service }`, and reshaped `BootCtl` around `run_mode()`/`set_run_mode()`, `reset()`, and `hand_off()`.
- **Breaking:** `tinyboot_core::traits` flattened; `BootClient` gone, app behaviour moved to `tinyboot_core::app::App`.
- **Breaking:** V103 + `system-flash` `BootCtl::new` now takes `(Pin, Level, u32)` for the external BOOT0 circuit; other combinations stay unit-arg.
- **Breaking:** removed `Storage::unlock()` — flash lock/unlock is now scoped per operation inside the HAL.
- Switched `ch32-metapac` to git for V00x support.
- Run-mode persistence split into variant-specific backends (`BOOT_MODE` register on V003 system-flash, RAM magic word elsewhere, placed at the last 4 bytes of RAM to avoid colliding with `qingke-rt`'s highcode-init flag).
- Non-HAL `bool` parameters converted to enums (`Duplex`, `Level`, `Pull`, `RunMode`, `BootSrc`).
- Dispatcher refactored for readability and reduced size (eliminated `.rodata` jump table; shared `frame.send` path).

### Fixed

- Dispatcher now flushes the transport after send — required for RS-485 / DXL TTL half-duplex.
- Ring buffer properly reset after flush.

## [v0.3.0] - 2026-04-15

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

## [v0.2.1] - 2026-03-25

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

## [v0.2.0] - 2026-03-20

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

## [v0.1.0] - 2026-03-20

Initial release.
