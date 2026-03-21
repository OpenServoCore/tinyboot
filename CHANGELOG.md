# Changelog

## [0.2.0] - 2026-03-20

### Changed

- **Breaking:** Verify command now carries `app_size` in the addr field
- **Breaking:** `BootMetaStore` trait: `trials_remaining()` replaced by `has_trials() -> bool`; `refresh()` takes an additional `app_size` parameter
- **Breaking:** `BootMetaStore::new()` replaced by `Default` impl (`BootMetaStore::default()`)
- CRC16 validation now covers only actual firmware bytes, not the entire flash region
- CLI only writes actual firmware data — no more 0xFF padding to fill the region
- App version read from end of binary (`flash[app_size-2..app_size]`) instead of end of flash region
- Linker script places `.tinyboot_version` after all other flash content (end of binary) instead of at end of flash region
- OB metadata expanded from 8 to 16 bytes (added app_size u32 field)
- System flash memory.x corrected to LENGTH=1920 (actual system flash size)

### Added

- `iwdg::feed()` in HAL — feeds the independent watchdog timer before OB erase in app-side `confirm()` to prevent watchdog reset during the critical OB erase+rewrite window
- `has_trials() -> bool` on `BootMetaStore` trait — simpler and avoids software popcount on targets without hardware support

### Optimized

- Startup assembly stripped to 20 bytes (from 88) — removed .data copy loop, .bss zero loop, and alignment padding since the bootloader uses no mutable statics
- Flash time reduced proportionally to firmware size (e.g. 5KB app on 16KB chip: ~8s vs full-region flash)
- CRC verification faster — only covers firmware bytes

## [0.1.0] - 2026-03-20

Initial release.
