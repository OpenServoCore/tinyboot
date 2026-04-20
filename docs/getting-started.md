# Getting started

A walkthrough for flashing the tinyboot bootloader + a demo app onto a CH32V003, then updating the app over UART. This is the expanded version of the [top-level README](https://github.com/OpenServoCore/tinyboot#quick-start-ch32v003) quick start, with more detail on each step and notes for the other supported chips.

## What you'll need

- A CH32V003 board (e.g. the CH32V003F4P6-R0 dev board, or a custom board). This guide uses PD5/PD6 for UART, which is the factory default.
- A WCH-LinkE programmer (for SWIO / one-wire debug) or equivalent wlink-supported probe.
- A USB-UART adapter wired to the MCU's UART pins (TX → RX, RX → TX, GND shared). For RS-485 / DXL TTL, see the [transports guide](transports.md).

If you're on a different chip (CH32V00x or CH32V103), the steps are the same — just swap the example directory. See the [chip notes](#chip-notes) below.

## 1. Install tools

```sh
# Rust nightly with the RISC-V target the examples use
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# WCH programming tool (flashes over WCH-LinkE)
cargo install wlink

# The tinyboot host CLI
cargo install tinyboot
```

The CH32V003 examples use `riscv32ec-unknown-none-elf`, which is a Tier 3 target that `-Zbuild-std` builds on the fly — nightly is required. CH32V103 examples use the stable `riscv32imc-unknown-none-elf` target.

## 2. Clone the repo

```sh
git clone https://github.com/OpenServoCore/tinyboot
cd tinyboot
```

## 3. Build and flash the bootloader

```sh
cd examples/ch32/v003/boot
cargo build --release
wlink flash --address 0x1FFFF000 target/riscv32ec-unknown-none-elf/release/boot
```

> [!IMPORTANT]
> After flashing system flash, power-cycle the board before continuing — a software reset is not sufficient to switch the CPU over to the new bootloader on some chips. The easiest way is `wlink set-power disable3v3 && wlink set-power enable3v3`.

At this point the chip runs the bootloader at power-on. With no valid app in user flash, it will sit and wait for a host.

## 4. Build the demo app

```sh
cd ../app
cargo build --release
```

This produces an ELF at `target/riscv32ec-unknown-none-elf/release/app`.

## 5. Flash the app over UART

Connect your USB-UART adapter to the MCU's UART pins, then:

```sh
tinyboot flash target/riscv32ec-unknown-none-elf/release/app --reset
```

`--reset` tells the bootloader to jump into the app once the flash and verify steps succeed. You should see the LED blink (TIM2-driven, ~1 Hz) — that's the app running.

If `--port` is omitted, the CLI probes USB serial ports by sending an Info request to each. If auto-detection fails, pass `--port /dev/ttyUSB0` (or the equivalent on your OS).

## 6. Verify it worked

```sh
tinyboot info
```

You should see something like:

```
capacity: 16320
erase_size: 64
boot_version: 0.4.0
app_version: 1.2.3
mode: 1
```

`mode: 1` means the app is running. `mode: 0` would mean the bootloader is running — you can switch the device into bootloader mode at any time:

```sh
tinyboot reset --bootloader
```

## Next steps

- [App integration](app-integration.md) — how to wire `poll()` and `confirm()` into your own app
- [Remote firmware updates](remote-updates.md) — the end-to-end OTA flow
- [Flash modes](flash-modes.md) — system-flash vs user-flash tradeoffs
- [Transports](transports.md) — RS-485, DXL TTL, alternate pins and baud rates
- [Troubleshooting](troubleshooting.md) — if something above didn't work

## Chip notes

### CH32V003 (this guide)

- System flash: `0x1FFFF000`, 1920 bytes.
- User flash: 16 KB, minus the last 64-byte META page.
- No BOOT_CTL circuit needed.

### CH32V00x (V002 / V004 / V005 / V006 / V007)

- System flash: `0x1FFF0000`, 3 KB + 256 B.
- Example directory: `examples/ch32/v00x/`.
- `wlink` auto-detects V006 / V007 together — this is expected.

### CH32V103

- System flash: `0x1FFFF000`, 2048 bytes.
- Example directory: `examples/ch32/v103/`.
- **Requires** an external BOOT_CTL circuit to switch between system flash (bootloader) and user flash (app) across a reset. See [GPIO-controlled boot mode selection](boot-ctl.md).
- The example uses `BootCtl::new(Pin::PB1, Level::High, 8000)` — adjust to your pin and RC timing.
