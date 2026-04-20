# tinyboot

[![CI](https://github.com/OpenServoCore/tinyboot/actions/workflows/ci.yml/badge.svg)](https://github.com/OpenServoCore/tinyboot/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-handbook-blue)](https://openservocore.github.io/tinyboot/)
[![MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)
[![Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE-APACHE)

A Rust bootloader for resource-constrained microcontrollers. Fits in the CH32V003's 1920-byte system flash with full trial boot, CRC16 app validation, and version reporting — leaving every byte of user flash free for your application.

![tinyboot demo](docs/demo.gif)

## Supported chips

| Family       | Status       |
| ------------ | ------------ |
| **CH32V003** | ✅ Supported |
| **CH32V00x** (V002 / V004 / V005 / V006 / V007) | ✅ Supported |
| **CH32V103** | ✅ Supported (needs a small BOOT0 circuit — see [boot-ctl](https://openservocore.github.io/tinyboot/boot-ctl.html)) |
| **CH32X03x** | 📋 Planned   |

Porting to a new MCU family is [a few hundred lines of glue](https://openservocore.github.io/tinyboot/porting.html).

## Quick start (CH32V003)

Five minutes from a blank chip to an app that updates itself over UART.

**1. Install the tools.**

```sh
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
cargo install wlink          # flash system flash via WCH-LinkE
cargo install tinyboot       # host CLI for UART flashing
```

**2. Clone the repo and flash the bootloader.**

```sh
git clone https://github.com/OpenServoCore/tinyboot
cd tinyboot/examples/ch32/v003/boot
cargo build --release
wlink flash --address 0x1FFFF000 target/riscv32ec-unknown-none-elf/release/boot
wlink set-power disable3v3 && wlink set-power enable3v3   # power-cycle
```

**3. Build and flash the demo app over UART.**

Connect a USB-UART adapter (TX ↔ PD6, RX ↔ PD5, GND shared), then:

```sh
cd ../app
cargo build --release
tinyboot flash target/riscv32ec-unknown-none-elf/release/app --reset
```

**4. Confirm it's running.**

```sh
tinyboot info
# capacity: 16320
# erase_size: 64
# boot_version: 0.4.0
# app_version: 1.2.3
# mode: 1            ← 1 means the app is running
```

LED should be blinking. To kick the device back into bootloader mode at any time:

```sh
tinyboot reset --bootloader
```

On CH32V00x or CH32V103, the flow is the same — swap the example directory. See [Getting Started](https://openservocore.github.io/tinyboot/getting-started.html) for chip-specific notes.

## Where to go next

The full documentation lives at **[openservocore.github.io/tinyboot](https://openservocore.github.io/tinyboot/)**. Highlights:

- [**Getting Started**](https://openservocore.github.io/tinyboot/getting-started.html) — expanded tutorial with more detail and per-chip notes
- [**CLI reference**](cli/README.md) — `tinyboot flash / info / erase / reset / bin`
- [**App integration**](https://openservocore.github.io/tinyboot/app-integration.html) — put `poll()` and `confirm()` into your own firmware
- [**Remote firmware updates**](https://openservocore.github.io/tinyboot/remote-updates.html) — end-to-end OTA flow
- [**Troubleshooting**](https://openservocore.github.io/tinyboot/troubleshooting.html) — things that go wrong and how to fix them
- [**Porting to a new MCU**](https://openservocore.github.io/tinyboot/porting.html) — four traits, one HAL
- [**Design notes**](https://openservocore.github.io/tinyboot/design.html) — motivation, the 1920-byte budget, `unsafe` policy

## Why tinyboot?

The CH32 factory bootloader is fixed to 115200 baud on PD5/PD6 with a sum-mod-256 checksum and no error reporting. [embassy-boot](https://github.com/embassy-rs/embassy/tree/main/embassy-boot) is a well-designed bootloader but needs ~8 KB of flash — half the V003's total. tinyboot fits a real protocol (CRC16, trial boot, configurable transport) into 1920 bytes so every byte of user flash is yours.

For the full story and how it fits in 1920 bytes, see the [design notes](https://openservocore.github.io/tinyboot/design.html).

## Project structure

```
lib/core/         tinyboot-core — boot state machine, protocol dispatcher
lib/protocol/     tinyboot-protocol — wire protocol, frame format, CRC16
ch32/             tinyboot-ch32 — HAL + platform
ch32/rt/          tinyboot-ch32-rt — minimal bootloader runtime
cli/              tinyboot — host CLI flasher
examples/ch32/    per-chip boot + app examples (also CI test targets)
docs/             user guide
```

## Contributing

Contributions are very welcome — especially new chip ports and transports. See the [contributing guide](https://openservocore.github.io/tinyboot/contributing.html) and please [open an issue](https://github.com/OpenServoCore/tinyboot/issues) before starting anything big.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
