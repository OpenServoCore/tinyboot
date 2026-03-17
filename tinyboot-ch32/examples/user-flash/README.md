# User Flash Example

Bootloader hosted in user flash alongside the application. The 16KB user flash
is partitioned between the bootloader (4KB) and the application (12KB).

This configuration has more room for features — defmt logging and trial-boot
are both enabled.

> **Note:** User-flash mode is primarily useful for debugging, since it
> allows enabling defmt and other features that don't fit in system flash.
> For production, prefer hosting the bootloader in system flash so the
> entire user flash is reserved for the application and no custom `memory.x`
> is needed for the app. See the [`system-flash`](../system-flash/) example.

## Memory Layout

```
 User Flash (0x08000000)
 ┌──────────────────────────────┐ 0x08000000
 │  Bootloader code (4KB - 64) │
 ├──────────────────────────────┤ 0x08000FC0
 │  Boot metadata (64 B)       │
 ├──────────────────────────────┤ 0x08001000
 │                              │
 │  Application (12KB)          │
 │                              │
 └──────────────────────────────┘ 0x08004000

 RAM (0x20000000)
 ┌──────────────────────────────┐ 0x20000000
 │  Boot request word (4 B)    │  ← NOLOAD, survives soft reset
 ├──────────────────────────────┤ 0x20000004
 │  Data / BSS / Stack (2KB-4) │
 └──────────────────────────────┘ 0x20000800
```

## Boot Request Mechanism

Without the `system-flash` feature, the hardware `BOOT_MODE` register is not
available. Instead, a **magic word** (`0xB007_CAFE`) is written to a reserved
4-byte region at the start of RAM. This region is placed in a `NOLOAD` linker
section so it is not zeroed on startup, preserving its value across soft resets.

Both the bootloader and the app must link `boot_request.x` to reserve this
word. The bootloader's `link.x` includes it directly; the app links it via
`-Tboot_request.x` in its `build.rs`.

## Features & Flash Size Impact

The bootloader budget here is **4032 bytes** (4KB - 64 for metadata). The
current configuration uses ~3116 bytes, leaving headroom for additional
features.

| Configuration            | Approximate Size | Notes                            |
|--------------------------|------------------|----------------------------------|
| Base (defmt + trial-boot)| ~3116 B          | Current example configuration    |
| - `defmt` logging        | Saves ~1000 B    | Removes RTT + format strings     |
| - `trial-boot`           | Saves ~200 B     | Removes boot state machine       |
| + RS-485 `tx_en`         | Adds ~44 B       | Uncomment in UsartConfig         |
| + `rx_pull: Pull::Up`    | Adds ~8 B        | Internal pull-up on RX pin       |

### Adjusting the partition

The 4KB/12KB split can be adjusted by changing:

1. `boot/memory.x` — `CODE`, `FLASH`, and `META` regions
2. `boot/src/main.rs` — `APP_BASE`, `APP_SIZE`, and `META_BASE` constants
3. `app/memory.x` — `FLASH` origin and length

All three must agree. The bootloader partition must be aligned to the flash
erase size (1KB on CH32V003).

## Building

```sh
# Check
cargo check -p boot
cargo check -p app

# Build and flash (requires probe-rs)
cargo run -p boot
cargo run -p app
```
