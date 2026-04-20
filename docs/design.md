# Design notes

Why tinyboot exists, how it fits in the CH32V003's 1920-byte system flash, and what `unsafe` it uses.

## Motivation

tinyboot was built for [OpenServoCore](https://github.com/OpenServoCore), where CH32V006-based servo boards need seamless firmware updates over the existing DXL TTL bus — no opening the shell, no debug probe, just flash over the same wire the servos already talk on.

The existing options didn't fit:

- **CH32 factory bootloader** — Fixed to 115200 baud on PD5/PD6 with no way to configure UART pins, baud rate, or TX-enable for RS-485. Uses a sum-mod-256 checksum that silently drops bad commands with no error response. No CRC verification, no trial boot, no boot state machine. See [ch32v003-bootloader-docs](https://github.com/basilhussain/ch32v003-bootloader-docs) for the reverse-engineered protocol details.
- **[embassy-boot](https://github.com/embassy-rs/embassy/tree/main/embassy-boot)** — A well-designed bootloader, but requires ~8KB of flash. That's half the V003's 16KB user flash, and doesn't fit in system flash at all. Not practical for MCUs with 16-32KB total.

I took it as a challenge to fit a proper bootloader — with a real protocol, CRC16 validation, trial boot, and configurable transport — into the CH32V003's 1920-byte system flash. The key inspiration was [rv003usb](https://github.com/cnlohr/rv003usb) by cnlohr, whose software USB implementation includes a 1920-byte bootloader in system flash. That project proved it was possible to fit meaningful code in that space, and showed me that the entire 16KB of user flash could be left free for the application.

## Design approach

tinyboot is structured as a library, not a monolithic binary. The core logic and protocol live in platform-agnostic crates; chip-specific details live in a single `tinyboot-{chip}` crate (`tinyboot-ch32` for CH32) with a `boot` module for bootloader binaries and an `app` module for applications.

To build your bootloader, you create a small crate with a `main.rs` that wires up your transport and calls `boot::run()` — see the [examples](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v003) for exactly this. The app module plugs into your application so it can confirm a successful boot and reboot into the bootloader on command, enabling fully remote firmware updates without physical access.

## How it fits in 1920 bytes

Beyond the usual Cargo profile tricks (`opt-level = "z"`, LTO, `codegen-units = 1`, `panic = "abort"`), fitting a real bootloader in 1920 bytes required more deliberate choices:

- **No HAL crates** — bare metal register access via PAC crates only; HAL abstractions are too expensive for this budget.
- **Custom runtime** — `tinyboot-ch32-rt` replaces `qingke-rt` in the bootloader; its startup is just GP/SP init and a jump to main (20 bytes of assembly instead of ~1.4KB of full runtime).
- **Symmetric frame format** — the same `Frame` struct is used for both requests and responses with one shared parse and format path, eliminating code duplication.
- **`repr(C)` frame with union data** — CRC is computed directly over the struct memory via pointer cast; no serialization step, no intermediate buffer.
- **`MaybeUninit` frame buffer** — the 76-byte `Frame` struct is reused every iteration without zero-initialization.
- **Bit-bang CRC16** — no lookup table, trades speed for ~512 bytes of flash savings.
- **Bit-clear state transitions** — forward state changes (Idle → Updating, trial consumption) flip 1→0 bits without erasing, avoiding a full erase + rewrite cycle.
- **Avoid `memset` / `memcpy`** — these pull in expensive `core` routines; manual byte loops and volatile writes keep the linker from dragging in library code.
- **`.write()` over `.modify()`** — register writes use direct writes instead of read-modify-write, saving the read and mask operations.
- **Aggressive code deduplication** — shared flash operation primitives across erase and write (see the flash HAL).

## Safety

The crates use `unsafe` in targeted places, primarily to meet the extreme size constraints of system flash (1920 bytes):

- **`repr(C)` unions and `MaybeUninit`** — zero-copy frame access and avoiding zero-initialization overhead.
- **`read_volatile` / `write_volatile`** — direct flash reads / writes, version reads, and boot request flag access.
- **`transmute`** — enum conversions (boot state) and function pointer cast for jump-to-address.
- **`from_raw_parts`** — zero-copy flash slice access in the storage layer.
- **Linker section attributes** — placing version data and boot metadata at fixed flash addresses.
- **`export_name` / `extern "C"`** — runtime entry points and linker symbol access for chip runtime integration.

These are deliberate trade-offs — safe alternatives would pull in extra code that doesn't fit. The `unsafe` is confined to data layout, memory access, and hardware boundaries; the bootloader state machine and protocol logic are safe Rust.
