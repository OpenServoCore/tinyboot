# tinyboot

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — start with the [top-level README](https://github.com/OpenServoCore/tinyboot#quick-start-ch32v003) and the [handbook](https://openservocore.github.io/tinyboot/).

Host-side CLI for flashing firmware to tinyboot devices over UART / RS-485.

## Install

```sh
cargo install tinyboot
```

Or from source:

```sh
cargo install --path cli
```

## Usage

### `tinyboot info`

Query device info (capacity, erase size, versions, mode).

```sh
tinyboot info [--port /dev/ttyUSB0] [--baud 115200]
```

### `tinyboot flash`

Flash firmware to device. Accepts ELF or raw binary files.

```sh
tinyboot flash firmware.elf [--port /dev/ttyUSB0] [--baud 115200] [--reset]
```

### `tinyboot erase`

Erase the entire app region.

```sh
tinyboot erase [--port /dev/ttyUSB0] [--baud 115200]
```

### `tinyboot reset`

Reset the device. Use `--bootloader` to reboot into the bootloader instead of the app.

```sh
tinyboot reset [--port /dev/ttyUSB0] [--baud 115200] [--bootloader]
```

### `tinyboot bin`

Convert an ELF to a flat binary (same extraction logic as `flash`).

```sh
tinyboot bin firmware.elf -o firmware.bin
```

## Auto-detection

If `--port` is omitted, the CLI probes USB serial ports (usbmodem, ttyACM, ttyUSB) by sending an Info command with a 100ms timeout. Non-USB serial ports are skipped. Both the bootloader and apps running `poll()` respond to Info, so auto-detection works in either mode.

## ELF handling

When given an ELF file, the CLI extracts ALLOC sections using physical addresses (LMA) from PT_LOAD segments. Sections named `.uninit*` are skipped. LMAs below `0x0800_0000` are adjusted by adding the CH32 flash base offset.

Raw binary files (no ELF magic) are used as-is.

## Logging

Use `-v` (debug) or `-vv` (trace) for protocol-level diagnostics:

```sh
tinyboot -v flash firmware.elf
tinyboot -vv flash firmware.elf
```

Or set `RUST_LOG` directly:

```sh
RUST_LOG=debug tinyboot flash firmware.elf
```
