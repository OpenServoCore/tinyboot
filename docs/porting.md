# Porting to a new MCU family

Adding a new chip within an existing family (e.g. another CH32 variant) is straightforward — add the register definitions to the existing HAL module and a feature flag. No new crates needed.

Porting to an entirely new MCU family (e.g. STM32) requires a parallel crate. The core crates (`tinyboot-core`, `tinyboot-protocol`, `tinyboot`) are platform-agnostic — you implement four traits and provide a minimal HAL. Here's what that looks like.

## 1. Create a `tinyboot-{chip}` crate

Mirror the layout of [`tinyboot-ch32`](https://github.com/OpenServoCore/tinyboot/tree/main/ch32):

- `src/hal/` — low-level register access: flash (unlock/erase/write/lock), GPIO (configure, set level), USART (init, blocking read/write/flush), RCC (enable peripherals), reset (system reset + optional jump).
- `src/platform/` — implementations of the four `tinyboot_core::traits` on top of the HAL.
- `src/boot.rs` and `src/app.rs` — thin bootloader and app entry points exposing the platform to user binaries.

### The four traits

| Trait           | What to implement                                                                                                                         |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Transport`     | Any `embedded_io::Read + Write` stream — UART, RS-485, USB, SPI, even WiFi or Bluetooth. The protocol doesn't care what carries the bytes |
| `Storage`       | `embedded_storage::NorFlash` (erase, write, read), plus `as_slice()` for zero-copy flash reads                                            |
| `BootMetaStore` | Read/write boot state, trial counter, app checksum, and app size from a reserved flash page (address from linker symbol)                  |
| `BootCtl`       | `run_mode()`/`set_run_mode()` for Service/HandOff intent across reset, `reset()` for software reset, `hand_off()` to boot the app         |

## 2. (Optional) Create a `tinyboot-{chip}-rt` crate

If your chip needs a custom `_start` + linker script to fit a small bootloader — [`tinyboot-ch32-rt`](https://github.com/OpenServoCore/tinyboot/tree/main/ch32/rt) exists for this reason on CH32 — ship one alongside. Otherwise the regular chip runtime is fine for the app.

## 3. Create an example workspace

Add `examples/{chip}/{variant}/` with boot + app binaries. Each provides a `memory.x` defining the five standard regions (`CODE`, `BOOT`, `APP`, `META`, `RAM`). The core linker scripts (`tb-boot.x`, `tb-app.x`, `tb-run-mode.x`) handle the rest.

### Linker region contract

All `memory.x` files define five standard regions. The crate linker scripts (`tb-boot.x`, `tb-app.x`) derive all `__tb_*` symbols from these regions — no magic addresses in `memory.x`.

| Region | Description                                         |
| ------ | --------------------------------------------------- |
| `CODE` | Execution mirror (VMA) of the binary's flash region |
| `BOOT` | Bootloader physical flash                           |
| `APP`  | Application physical flash                          |
| `META` | Boot metadata (last flash page)                     |
| `RAM`  | SRAM                                                |

## What you get for free

The entire protocol (frame format, CRC, sync, commands), the boot state machine (Idle / Updating / Validating transitions, trial boot logic, app validation), the CLI, and the host-side flashing workflow all work unchanged. You only write the chip-specific glue.

## Before starting a port

Please [open an issue](https://github.com/OpenServoCore/tinyboot/issues) so we can discuss the approach. Some chip families have surprises (boot-pin muxing, flash write granularity, clock domain quirks) that we've already run into on CH32 and can share context on.
