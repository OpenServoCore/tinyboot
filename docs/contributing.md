# Contributing

Thanks for your interest in tinyboot. This page covers the dev setup, test procedures, and workflow conventions.

## Before starting

- For anything bigger than a typo fix, please [open an issue](https://github.com/OpenServoCore/tinyboot/issues) first so we can discuss the approach.
- New chip ports are especially welcome — see the [porting guide](porting.md) for the trait surface you'd need to implement.

## Workspace layout

```
lib/                         platform-agnostic core
  core/                      tinyboot-core
  protocol/                  tinyboot-protocol
ch32/                        CH32 HAL + platform
  rt/                        tinyboot-ch32-rt (minimal bootloader runtime)
cli/                         tinyboot host CLI
examples/ch32/v003/          V003 boot + app (CI testbed)
examples/ch32/v00x/          V00x boot + app
examples/ch32/v103/          V103 boot + app
docs/                        user-facing documentation
```

Each directory is its own Cargo workspace with an independent `Cargo.lock`. A "clean compile" of the project therefore means wiping `target/` under every workspace — see below.

## Rust toolchain

- **Library crates (`lib/`, `cli/`)** — stable Rust 1.85+, edition 2024.
- **CH32 example binaries** — nightly, for `-Zbuild-std` on `riscv32ec-unknown-none-elf` (V003 / V00x) or the stable `riscv32imc-unknown-none-elf` target (V103).

Each example workspace pins its toolchain via `rust-toolchain.toml`.

## Running tests

```sh
# Unit tests for the platform-agnostic crates
cd lib && cargo test

# Build every example to make sure all feature combinations compile
cd examples/ch32/v003 && cargo build --release
cd examples/ch32/v00x && cargo build --release
cd examples/ch32/v103 && cargo build --release

# Host CLI
cd cli && cargo test
```

CI runs a matrix across chip variants and flash modes. Match that before opening a PR.

## Clean compile

When hunting size regressions or build issues, a "clean compile" means removing `target/` from **every** workspace. A leftover `target/` in one workspace can mask issues in another:

```sh
find . -type d -name target -prune -exec rm -rf {} +
```

Then rebuild the affected workspaces.

## Hardware validation

Some changes (particularly to flash, BootCtl, or the RS-485 transport) can't be caught by unit tests and need on-hardware validation.

### Integration test checklist (user-flash mode)

This is the acceptance test we run before merging flash-touching changes on CH32V003 / V103 in user-flash mode:

1. Erase user flash via `wlink`.
2. Build and flash the bootloader to the `BOOT` region.
3. Power-cycle the board.
4. Confirm `tinyboot info` reports `mode = 0` and the expected `boot_version`.
5. Build the app.
6. Flash the app via `tinyboot flash <app> --reset`.
7. Confirm the app is running (LED blinks, `tinyboot info` reports `mode = 1`).
8. Re-flash a different app version to exercise the update flow end-to-end.
9. Trigger bootloader re-entry via `tinyboot reset --bootloader`.
10. Re-flash the original app, confirm it still runs.
11. Simulate an app that never confirms — boot should fall back to the bootloader after trials run out.
12. Simulate a power loss mid-flash (disconnect power during a write); confirm recovery.
13. Confirm `META` is in the expected location post-update.

> [!NOTE]
> On CH32V103 in user-flash mode with the BOOT_CTL RC network installed, temporarily disconnect the PB1 → BOOT0 trace before running this procedure. A soft reset can otherwise latch BOOT0 HIGH and route you into system flash.

### System-flash mode

System-flash validation follows the same shape but uses `wlink` to write to the system flash address (`0x1FFFF000` on V003 / V103, `0x1FFF0000` on V00x). After writing system flash, always power-cycle before testing.

## Commit and PR conventions

- Small, focused commits. Commit messages in imperative mood (e.g. "Add V00x feature flag", "Fix FTPG partial page write").
- A PR covering a behavior change should note how you validated it (unit test added, hardware procedure run, both).
- Keep docs changes in the same PR as the code change they describe, unless the docs are big enough to deserve their own review.

## Releases

Releases are tagged `vX.Y.Z` across the whole repo — all crates share a version. `tinyboot-ch32` stays git-only (not published to crates.io) while it depends on an unreleased `ch32-metapac`. The rest (`tinyboot-core`, `tinyboot-protocol`, `tinyboot`, `tinyboot-ch32-rt`) publish to crates.io.
