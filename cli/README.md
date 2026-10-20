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

Every command that talks to a device takes `--port`, `--baud` and `--uid`. `--uid` picks one device on a shared bus by its unique id, in hex as `tinyboot discover` prints it. A unique prefix of whole bytes is enough. Without `--uid` the CLI selects the only device on the bus, and stops with an error when more than one answers. A device without addressing is used as it is. See the [shared bus guide](https://openservocore.github.io/tinyboot/shared-bus.html).

### `tinyboot discover`

List the addressable devices on the bus by unique id, with mode, versions and compatibility key. With `--uid`, only the devices under that prefix.

```sh
tinyboot discover [--port /dev/ttyUSB0] [--baud 115200] [--uid 10]
```

### `tinyboot info`

Query device info (capacity, erase size, versions, mode).

```sh
tinyboot info [--port /dev/ttyUSB0] [--baud 115200] [--uid 1032]
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

### `tinyboot baud`

Store the baud the bootloader runs at from its next start, then reset the device back into its current mode. In bootloader mode the CLI reconnects at the new rate to check that the bootloader answers. `default` restores the bootloader's build default.

```sh
tinyboot baud 1000000 [--port /dev/ttyUSB0] [--baud 115200]
tinyboot baud default [--port /dev/ttyUSB0] [--baud 1000000]
```

### `tinyboot bin`

Convert an ELF to a flat binary (same extraction logic as `flash`).

```sh
tinyboot bin firmware.elf -o firmware.bin
```

## Auto-detection

If `--port` is omitted, the CLI probes USB serial ports (usbmodem, ttyACM, ttyUSB) by sending a `Select` with an empty prefix and listening for any reply. Non-USB serial ports are skipped. Addressable devices answer with their id, others with `Unsupported`, in the bootloader and in apps running `poll()` alike, so auto-detection works in either mode.

## Single-wire bus echo

On a single-wire bus where the host's TX and RX are tied to the data line (DXL chains, most RS-485 hookups), the host hears its own request frame before the device replies. The CLI skips any frame whose status is `Request` — devices never reply with that status — so single-wire setups work without extra flags.

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
