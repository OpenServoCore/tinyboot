# tinyboot-ch32-rt

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — see the main README to get started.

Minimal bootloader runtime for CH32. Ships a tiny `_start` (GP/SP init + jump to `main`) and a companion `link.x` script for bootloader binaries that can't afford `qingke-rt` — critical when the bootloader has to fit in the ~2 KB system-flash region.

## When to use

- **Bootloader binaries** — use this crate; keeps startup overhead to ~20 bytes.
- **Application binaries** — use `qingke-rt` instead. Do not depend on both; their `_start` symbols collide at link time.

## What it skips

No `.data` copy, no `.bss` zeroing, no interrupt vector table. Safe for the tinyboot bootloader because it:

- Has no initialized statics (no `.data`).
- Has no zero-init statics (no `.bss`).
- Polls rather than enabling interrupts.
- Only runs from power-on reset (SRAM is undefined then anyway).

## Usage

```toml
[dependencies]
tinyboot-ch32-rt = "0.3"
```

```rust
#![no_std]
#![no_main]

use panic_halt as _;
use tinyboot_ch32_rt as _;  // pulls in _start and link.x

#[unsafe(export_name = "main")]
fn main() -> ! {
    // your bootloader
}
```

`link.x` is added to the link search path automatically by this crate's `build.rs`. Include it alongside the crate-specific `tb-boot.x`:

```rust
// build.rs
println!("cargo:rustc-link-arg=-Ttb-boot.x");
println!("cargo:rustc-link-arg=-Ttb-run-mode.x");
```

See [`examples/ch32/v003/boot`](../../examples/ch32/v003/boot/) and [`examples/ch32/v103/boot`](../../examples/ch32/v103/boot/) for complete bootloader binaries.
