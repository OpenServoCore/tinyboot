# System Flash Example

Bootloader hosted in the CH32V003's 1920-byte system flash region, leaving the
entire 16KB user flash available for applications.

> **Note:** This example targets the CH32V003 which has 1920 bytes of system
> flash. Other variants in the CH32V00x family (CH32V002, V004, V005, V006,
> V007, M007) have **3KB + 256 bytes** of system flash, making them
> significantly roomier for a system-flash bootloader. Newer CH32 families
> may have even more. Adjust `memory.x` accordingly for your target chip.

## Memory Layout

```
 System Flash (0x1FFFF000)
 ┌──────────────────────────────┐ 0x1FFFF000
 │  Bootloader code (1856 B)   │
 ├──────────────────────────────┤ 0x1FFFFCC0
 │  Boot metadata (64 B)       │
 └──────────────────────────────┘ 0x1FFFF780

 User Flash (0x08000000)
 ┌──────────────────────────────┐ 0x08000000
 │                              │
 │  Application (16KB)          │
 │                              │
 └──────────────────────────────┘ 0x08004000

 RAM (0x20000000)
 ┌──────────────────────────────┐ 0x20000000
 │  Data / BSS / Stack (2KB)   │
 └──────────────────────────────┘ 0x20000800
```

## Boot Request Mechanism

Uses the hardware `BOOT_MODE` register in the CH32V003 flash controller
(`STATR`). The app sets this bit and triggers a soft reset to re-enter the
bootloader. No RAM reservation is needed.

## Features & Flash Size Impact

The system flash budget is **1856 bytes** (1920 - 64 for metadata). Every
feature and configuration choice matters at this scale.

| Configuration            | Approximate Size | Notes                            |
|--------------------------|------------------|----------------------------------|
| Base bootloader          | ~1748 B          | Full-duplex USART, RS-485 tx_en  |
| + `defmt` logging        | —                | Does not fit in system flash     |
| + `trial-boot`           | —                | Does not fit in system flash     |
| - RS-485 `tx_en`         | Saves ~44 B      | Remove if not using RS-485       |
| - `rx_pull`              | Saves ~8 B       | Use `Pull::None` if not needed   |
| Half-duplex USART        | Saves ~20 B      | Single-wire mode                 |

To stay within the 1856-byte budget:

- **No defmt** — logging infrastructure alone exceeds the available space.
- **No trial-boot** — the state machine adds too much code.
- Minimize USART configuration (remove `tx_en` if not using RS-485).
- Build with `opt-level = "z"` and `lto = true` (set in workspace profile).

## Building

```sh
# Check
cargo check -p boot
cargo check -p app

# Build and flash (requires probe-rs)
cargo run -p boot
cargo run -p app
```
