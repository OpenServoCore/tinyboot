# tinyboot

[![CI](https://github.com/OpenServoCore/tinyboot/actions/workflows/ci.yml/badge.svg)](https://github.com/OpenServoCore/tinyboot/actions/workflows/ci.yml)
[![tinyboot](https://img.shields.io/crates/v/tinyboot?label=tinyboot)](https://crates.io/crates/tinyboot)
[![tinyboot-core](https://img.shields.io/crates/v/tinyboot-core?label=tinyboot-core)](https://crates.io/crates/tinyboot-core)
[![tinyboot-ch32](https://img.shields.io/crates/v/tinyboot-ch32?label=tinyboot-ch32)](https://crates.io/crates/tinyboot-ch32)
[![tinyboot-ch32-rt](https://img.shields.io/crates/v/tinyboot-ch32-rt?label=tinyboot-ch32-rt)](https://crates.io/crates/tinyboot-ch32-rt)
[![MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)
[![Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE-APACHE)

Rust bootloader for resource-constrained microcontrollers. Fits in the CH32V003's 1920-byte system flash with full trial boot, CRC16 app validation, and version reporting — leaving all but one page of user flash for the application (64 bytes reserved for boot metadata on V003; page size varies by chip).

![tinyboot demo](docs/demo.gif)

## Chip Compatibility

tinyboot currently supports **UART / RS-485** transport. The table below tracks chip support status. Chips with hardware boot pins (e.g. BOOT0/BOOT1) require a small external circuit to allow firmware-controlled boot mode switching. Please see [GPIO-Controlled Boot Mode Selection](docs/boot-ctl.md) document for more information.

✅ Verified | ❓ Untested (same die, likely works — volunteer needed) | 📋 Planned

| Chip         | Feature Flag   | System Flash              | Status | Blocker                                |
| ------------ | -------------- | ------------------------- | ------ | -------------------------------------- |
| CH32V003F4P6 | `ch32v003f4p6` | `0x1FFFF000` (1920B)      | ✅     | --                                     |
| CH32V003A4M6 | `ch32v003a4m6` | `0x1FFFF000` (1920B)      | ❓     | --                                     |
| CH32V003F4U6 | `ch32v003f4u6` | `0x1FFFF000` (1920B)      | ❓     | --                                     |
| CH32V003J4M6 | `ch32v003j4m6` | `0x1FFFF000` (1920B)      | ❓     | --                                     |
| CH32V103C6T6 | `ch32v103c6t6` | `0x1FFFF000` (2048B)      | ❓     | --                                     |
| CH32V103C8T6 | `ch32v103c8t6` | `0x1FFFF000` (2048B)      | ✅     | --                                     |
| CH32V103C8U6 | `ch32v103c8u6` | `0x1FFFF000` (2048B)      | ❓     | --                                     |
| CH32V103R8T6 | `ch32v103r8t6` | `0x1FFFF000` (2048B)      | ❓     | --                                     |
| CH32V002X4X6 | `ch32v002x4x6` | `0x1FFF0000` (3KB + 256B) | 📋     | `flash_v00x` HAL ([#29][ch32-data-29]) |
| CH32V004X6X1 | `ch32v004x6x1` | `0x1FFF0000` (3KB + 256B) | 📋     | `flash_v00x` HAL ([#29][ch32-data-29]) |
| CH32V005X6X6 | `ch32v005x6x6` | `0x1FFF0000` (3KB + 256B) | 📋     | `flash_v00x` HAL ([#29][ch32-data-29]) |
| CH32V006X8X6 | `ch32v006x8x6` | `0x1FFF0000` (3KB + 256B) | 📋     | `flash_v00x` HAL ([#29][ch32-data-29]) |
| CH32V007X8X6 | `ch32v007x8x6` | `0x1FFF0000` (3KB + 256B) | 📋     | `flash_v00x` HAL ([#29][ch32-data-29]) |
| CH32X033F8P6 | `ch32x033f8p6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X034F8P6 | `ch32x034f8p6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X034F8U6 | `ch32x034f8u6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X035C8T6 | `ch32x035c8t6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X035F7P6 | `ch32x035f7p6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X035F8U6 | `ch32x035f8u6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X035G8R6 | `ch32x035g8r6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X035G8U6 | `ch32x035g8u6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |
| CH32X035R8T6 | `ch32x035r8t6` | `0x1FFF0000` (3KB + 256B) | 📋     | --                                     |

## Features

- **Tiny** — Fits in 1920 bytes of CH32V003 system flash, leaving all 16KB user flash for the application
- **CRC16 validation** — Every frame is CRC16-CCITT protected; app image is verified end-to-end after flashing
- **Trial boot** — New firmware gets a limited number of boot attempts; if the app doesn't confirm, the bootloader takes over automatically
- **Boot state machine** — Idle / Updating / Validating lifecycle tracked in a reserved flash page with forward-only bit transitions (no erase needed for state advances)
- **Version reporting** — Boot and app versions packed into flash, queryable over the wire
- **Configurable transport** — The protocol runs over any `embedded_io::Read + Write` stream. The CH32 implementation supports UART with configurable pins, baud rate, and optional TX-enable for RS-485 / DXL TTL, but the core is transport-agnostic — USB, SPI, Bluetooth, or WiFi would work just as well
- **App-side integration** — The app can confirm a successful boot and request bootloader entry over the wire, enabling fully remote firmware updates without physical access
- **Library, not binary** — Build your bootloader by creating a small crate that wires up your specific hardware; the core logic is reusable across chips
- **Modular and portable** — Platform-agnostic core with four traits (`Transport`, `Storage`, `BootMetaStore`, `BootCtl`) that you implement for your MCU; the protocol, state machine, and CLI work unchanged

## Getting Started

1. **Build your bootloader** — create a small crate with a `main.rs` that configures your transport and calls `run()`. See the [CH32V003 example](examples/ch32/v003/boot/) — the entire bootloader main is just a `boot_version!()` macro and a `run()` call with USART config.

2. **Flash the bootloader** to system flash using [wlink](https://github.com/ch32-rs/wlink):

   ```sh
   wlink flash --address 0x1FFFF000 target/riscv32ec-unknown-none-elf/release/boot
   ```

3. **Install the CLI** and flash your app over UART:

   ```sh
   cargo install tinyboot
   tinyboot flash target/riscv32ec-unknown-none-elf/release/app --reset
   ```

## Project Structure

```
lib/                        Platform-agnostic core
  core/                     tinyboot-core — boot state machine, protocol dispatcher
  protocol/                 tinyboot-protocol — wire protocol, frame format, CRC16

ch32/                       CH32 implementation
  src/                      tinyboot-ch32 — HAL + platform (boot + app)
  rt/                       tinyboot-ch32-rt — tiny bootloader runtime

cli/                        tinyboot — host CLI flasher

examples/ch32/v003/         CH32V003 boot + app examples
examples/ch32/v103/         CH32V103 boot + app examples
```

| Crate                                    | Description                                                                                         |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [`tinyboot-core`](lib/core/)             | Platform-agnostic bootloader core (protocol dispatcher, boot state machine, app validation)         |
| [`tinyboot-protocol`](lib/protocol/)     | Wire protocol (frame format, CRC16, commands)                                                       |
| [`tinyboot-ch32`](ch32/)                 | CH32 HAL and tinyboot platform — use `boot` for bootloader binaries, `app` for application binaries |
| [`tinyboot-ch32-rt`](ch32/rt/)           | Minimal CH32 runtime for bootloader binaries that can't afford full `qingke-rt`                     |
| [`tinyboot`](cli/)                       | CLI firmware flasher over UART                                                                      |

## Rust Version

The workspaces use **edition 2024**.

- **Library crates and CLI** — stable Rust 1.85+
- **CH32 examples** (bootloader and app binaries) — nightly, for `-Zbuild-std` on `riscv32ec-unknown-none-elf` (V003) or standard `riscv32imc-unknown-none-elf` (V103)

## Linker Region Contract

All memory.x files define five standard regions: `CODE`, `BOOT`, `APP`, `META`, `RAM`. The crate linker scripts (`tb-boot.x`, `tb-app.x`) derive all `__tb_*` symbols from these regions — no magic addresses in memory.x.

| Region | Description                                         |
| ------ | --------------------------------------------------- |
| `CODE` | Execution mirror (VMA) of the binary's flash region |
| `BOOT` | Bootloader physical flash                           |
| `APP`  | Application physical flash                          |
| `META` | Boot metadata (last flash page)                     |
| `RAM`  | SRAM                                                |

## Porting to a New MCU Family

Adding a new chip within an existing family (e.g. another CH32 variant) is straightforward — add the register definitions to the existing HAL module and a feature flag. No new crates needed.

Porting to an entirely new MCU family (e.g. STM32) requires a parallel crate. The core crates (`tinyboot-core`, `tinyboot-protocol`, `tinyboot`) are platform-agnostic — you implement four traits and provide a minimal HAL. Here's what that looks like:

### 1. Create a `tinyboot-{chip}` crate

Mirror the layout of `tinyboot-ch32`:

- `src/hal/` — low-level register access: flash (unlock/erase/write/lock), GPIO (configure, set level), USART (init, blocking read/write/flush), RCC (enable peripherals), reset (system reset + optional jump).
- `src/platform/` — implementations of the four `tinyboot_core::traits` on top of the HAL.
- `src/boot.rs` and `src/app.rs` — thin bootloader and app entry points exposing the platform to user binaries.

The four traits from `tinyboot_core::traits`:

| Trait           | What to implement                                                                                                                         |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Transport`     | Any `embedded_io::Read + Write` stream — UART, RS-485, USB, SPI, even WiFi or Bluetooth. The protocol doesn't care what carries the bytes |
| `Storage`       | `embedded_storage::NorFlash` (erase, write, read), plus `as_slice()` for zero-copy flash reads                                            |
| `BootMetaStore` | Read/write boot state, trial counter, app checksum, and app size from a reserved flash page (address from linker symbol)                  |
| `BootCtl`       | `run_mode()`/`set_run_mode()` for Service/HandOff intent across reset, `reset()` for software reset, `hand_off()` to boot the app         |

### 2. (Optional) Create a `tinyboot-{chip}-rt` crate

If your chip needs a custom `_start` + linker script to fit a small bootloader (`tinyboot-ch32-rt` exists for this reason on CH32), ship it alongside; otherwise the regular chip runtime is fine for the app.

### 3. Create an example workspace

Add `examples/{chip}/{variant}/` with boot + app binaries. Each provides a `memory.x` defining the five standard regions (`CODE`, `BOOT`, `APP`, `META`, `RAM`). The core linker scripts (`tb-boot.x`, `tb-app.x`, `tb-run-mode.x`) handle the rest.

### What you get for free

The entire protocol (frame format, CRC, sync, commands), the boot state machine (Idle/Updating/Validating transitions, trial boot logic, app validation), the CLI, and the host-side flashing workflow all work unchanged. You only write the chip-specific glue.

## Why tinyboot?

I built tinyboot for [OpenServoCore](https://github.com/OpenServoCore), where CH32V006-based servo boards need seamless firmware updates over the existing DXL TTL bus — no opening the shell, no debug probe, just flash over the same wire the servos already talk on.

The existing options didn't fit:

- **CH32 factory bootloader** — Fixed to 115200 baud on PD5/PD6 with no way to configure UART pins, baud rate, or TX-enable for RS-485. Uses a sum-mod-256 checksum that silently drops bad commands with no error response. No CRC verification, no trial boot, no boot state machine. See [ch32v003-bootloader-docs](https://github.com/basilhussain/ch32v003-bootloader-docs) for the reverse-engineered protocol details.

- **[embassy-boot](https://github.com/embassy-rs/embassy/tree/main/embassy-boot)** — A well-designed bootloader, but requires ~8KB of flash. That's half the V003's 16KB user flash, and doesn't fit in system flash at all. Not practical for MCUs with 16-32KB total.

I took it as a challenge to fit a proper bootloader — with a real protocol, CRC16 validation, trial boot, and configurable transport — into the CH32V003's 1920-byte system flash. The key inspiration was [rv003usb](https://github.com/cnlohr/rv003usb) by cnlohr, whose software USB implementation includes a 1920-byte bootloader in system flash. That project proved it was possible to fit meaningful code in that space, and showed me that the entire 16KB of user flash could be left free for the application.

### How it fits in 1920 bytes

Beyond the usual Cargo profile tricks (`opt-level = "z"`, LTO, `codegen-units = 1`, `panic = "abort"`), fitting a real bootloader in 1920 bytes required some more deliberate choices:

- **No HAL crates** — bare metal register access via PAC crates only; HAL abstractions are too expensive for this budget
- **Custom runtime** — `tinyboot-ch32-rt` replaces qingke-rt in the bootloader; its startup is just GP/SP init and a jump to main (20 bytes of assembly instead of ~1.4KB of full runtime)
- **Symmetric frame format** — the same `Frame` struct is used for both requests and responses with one shared parse and format path, eliminating code duplication
- **`repr(C)` frame with union data** — CRC is computed directly over the struct memory via pointer cast; no serialization step, no intermediate buffer
- **`MaybeUninit` frame buffer** — the 76-byte `Frame` struct is reused every iteration without zero-initialization
- **Bit-bang CRC16** — no lookup table, trades speed for ~512 bytes of flash savings
- **Bit-clear state transitions** — forward state changes (Idle→Updating, trial consumption) flip 1→0 bits without erasing, avoiding the cost of a full erase+rewrite cycle
- **Avoid `memset`/`memcpy`** — these pull in expensive core routines; manual byte loops and volatile writes keep the linker from dragging in library code
- **`.write()` over `.modify()`** — register writes use direct writes instead of read-modify-write, saving the read and mask operations
- **Aggressive code deduplication** — shared flash operation primitives across erase and write (see the flash HAL)

### Design approach

tinyboot is structured as a library, not a monolithic binary. The core logic and protocol live in platform-agnostic crates; chip-specific details live in a single `tinyboot-{chip}` crate (`tinyboot-ch32` for CH32) with a `boot` module for bootloader binaries and an `app` module for applications. To build your bootloader, you create a small crate with a `main.rs` that wires up your transport and calls `boot::run()` — see the [examples](examples/ch32/v003/) for exactly this. The app module plugs into your application so it can confirm a successful boot and reboot into the bootloader on command, enabling fully remote firmware updates without physical access.

## Safety

The crates use `unsafe` in targeted places, primarily to meet the extreme size constraints of system flash (1920 bytes):

- **`repr(C)` unions and `MaybeUninit`** — zero-copy frame access and avoiding zero-initialization overhead
- **`read_volatile` / `write_volatile`** — direct flash reads/writes, version reads, and boot request flag access
- **`transmute`** — enum conversions (boot state) and function pointer cast for jump-to-address
- **`from_raw_parts`** — zero-copy flash slice access in the storage layer
- **Linker section attributes** — placing version data and boot metadata at fixed flash addresses
- **`export_name` / `extern "C"`** — runtime entry points and linker symbol access for chip runtime integration

These are deliberate trade-offs — safe alternatives would pull in extra code that doesn't fit. The unsafe is confined to data layout, memory access, and hardware boundaries; the bootloader state machine and protocol logic are safe Rust.

## Contributing

Contributions are welcome — especially new chip ports and transport implementations. If you're thinking about adding support for a new MCU family, the [Porting to a New MCU Family](#porting-to-a-new-mcu-family) section above covers the trait surface you'd need to implement.

Please [open an issue](https://github.com/OpenServoCore/tinyboot/issues) before starting a large PR so we can discuss the approach.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

[ch32-data-29]: https://github.com/ch32-rs/ch32-data/pull/29
