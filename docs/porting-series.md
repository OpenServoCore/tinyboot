# Adding a new chip series

This guide covers adding a new chip series within an existing family (e.g. adding the V00x series alongside V003 in the CH32 family). A new series typically has different peripheral versions and may require new HAL implementation files.

If you're porting to an entirely new MCU family (e.g. STM32), see [Porting to a new MCU family](porting.md) instead.

## Terminology

| Term       | Example                                      | Scope                                     |
| ---------- | -------------------------------------------- | ----------------------------------------- |
| **Family** | CH32, STM32                                  | One `tinyboot-{family}` crate             |
| **Series** | V003, V00x, V103                             | Chips sharing peripheral register layouts |
| **Variant**| CH32V003F4P6, CH32V006X8X6                   | A specific chip (package, flash/RAM size) |

## Quick orientation

The series-specific pieces in tinyboot are:

| What                   | Where                                         | Purpose                                                |
| ---------------------- | --------------------------------------------- | ------------------------------------------------------ |
| Feature flags          | `ch32/Cargo.toml`                             | Selects the right `ch32-metapac` register definitions  |
| Compile-error guard    | `ch32/src/lib.rs`                             | Ensures exactly one variant is selected                |
| HAL modules            | `ch32/src/hal/{flash,rcc,afio}/`              | Series-level register access, routed by `cfg`          |
| `build.rs`             | `ch32/build.rs`                               | Auto-detects peripheral versions from metapac metadata |
| Memory layouts         | `examples/ch32/{series}/{boot,app}/memory_x/` | Linker scripts with flash/RAM sizes from the datasheet |
| Example Cargo features | `examples/ch32/{series}/{boot,app}/Cargo.toml`| Wire feature flags through to `tinyboot-ch32`          |

## Step 1: Add feature flags

In `ch32/Cargo.toml`, add a feature for each variant in the new series:

```toml
[features]
# ... existing variants ...
ch32v006x8x6 = ["ch32-metapac/ch32v006x8x6"]
ch32v007x8x6 = ["ch32-metapac/ch32v007x8x6"]
```

Then add them to the compile-error guard in `ch32/src/lib.rs`:

```rust
#[cfg(not(any(
    // ... existing variants ...
    feature = "ch32v006x8x6",
    feature = "ch32v007x8x6",
)))]
compile_error!("No chip variant selected. ...");
```

## Step 2: Check HAL compatibility

The `build.rs` reads peripheral metadata from `ch32-metapac` and emits `cfg` flags like `flash_v0`, `rcc_v00x`, `afio_v0`, etc. To see which versions your new series gets, run a verbose build from the `ch32/` directory:

```bash
cargo build --features ch32v006x8x6 -vv 2>&1 | grep 'cargo:cfgs='
```

This prints a line like `cargo:cfgs=flash_v00x,rcc_v00x,afio_v0,...` — compare it against an existing series to see what differs.

If every peripheral version already has a HAL implementation, no changes are needed — the existing `cfg_attr` routing handles it.

If the metapac reports a **new** version (e.g. `flash_v00x`), you'll need to:

1. Create a new implementation file (e.g. `ch32/src/hal/flash/v00x.rs`) — copy the nearest existing version and adapt register access.
2. Add a `cfg_attr` line to the module's `mod.rs`:

```rust
// ch32/src/hal/flash/mod.rs
#[cfg_attr(flash_v0, path = "v0.rs")]
#[cfg_attr(flash_v00x, path = "v00x.rs")]   // new
#[cfg_attr(flash_v1, path = "v1.rs")]
mod family;
```

Repeat for `rcc/` and `afio/` if their versions differ too.

### Boot mode selection

The flash controller version also determines how the bootloader selects its flash region and persists run-mode across resets. The `build.rs` derives this automatically from the `boot_pin` flag (which itself comes from the flash version):

**Register-based** (`flash_v0`, `flash_v00x` — e.g. V003, V00x): The flash controller has a `BOOT_MODE` register that indicates which flash region the chip booted from. When `system-flash` is enabled, `run_mode` is read directly from this register (`run_mode_mode`), and the boot source is latched by writing `BOOT_MODE` before reset (`boot_src_mode`). No external hardware needed.

**Pin-based** (`flash_v1` — e.g. V103): The chip samples hardware BOOT0/BOOT1 pins at reset to choose the flash region. Run-mode is persisted as a magic word in RAM instead (`run_mode_ram`), and boot source is controlled by driving a GPIO connected to an external RC/flip-flop circuit on the BOOT0 pin (`boot_src_gpio`). The `BootCtl::new()` constructor takes pin, level, and delay parameters for this.

The `build.rs` also emits `split_sysflash` for pin-based chips with `system-flash` enabled, because the system flash on these chips is split into two regions by option bytes — flash HAL functions are placed in a second section (`.text2`) to fit.

If the new series introduces a flash controller version that doesn't fit either pattern, you'll need to add new `run_mode` and `boot_src` implementations under `ch32/src/platform/boot_ctl/` and update the `build.rs` logic accordingly.

## Step 3: Write the memory layout

Look up each variant's datasheet for:

- **User flash** base address, total size, and page size
- **System flash** base address and layout (if system-flash boot is supported)
- **RAM** base address and size

Create a `memory.x` linker script for each flash mode. When variants in the series have different flash/RAM sizes, each variant gets its own `memory.x` directory (see Step 4).

For example, CH32V002 (16K flash, 4K RAM, 256-byte pages):

```ld
/* user-flash.x */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 4K - 4
    CODE : ORIGIN = 0x00000000, LENGTH = 2K
    BOOT : ORIGIN = 0x08000000, LENGTH = 2K
    APP  : ORIGIN = 0x08000000 + 2K, LENGTH = 14K - 256
    META : ORIGIN = 0x08000000 + 16K - 256, LENGTH = 256
}
```

Versus CH32V006 (62K flash, 8K RAM, 256-byte pages):

```ld
/* user-flash.x */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 8K - 4
    CODE : ORIGIN = 0x00000000, LENGTH = 2K
    BOOT : ORIGIN = 0x08000000, LENGTH = 2K
    APP  : ORIGIN = 0x08000000 + 2K, LENGTH = 60K - 256
    META : ORIGIN = 0x08000000 + 62K - 256, LENGTH = 256
}
```

The `META` region must be exactly one flash page, placed at the end of flash. `RAM` reserves 4 bytes at the top for the `__tb_run_mode` word — this is needed for all user-flash builds, as well as system-flash on pin-based chips (where run-mode is persisted in RAM rather than the `BOOT_MODE` register).

## Step 4: Add the example

Each variant gets its own `memory.x` directory. The `build.rs` selects by variant feature:

```
examples/ch32/v00x/
├── boot/
│   ├── memory_x/
│   │   ├── ch32v002x4x6/
│   │   │   ├── system-flash.x
│   │   │   └── user-flash.x
│   │   └── ch32v006x8x6/
│   │       ├── system-flash.x
│   │       └── user-flash.x
│   ├── build.rs
│   └── Cargo.toml
└── app/
    └── (same structure)
```

The `build.rs` looks up the active variant:

```rust
const CHIPS: &[&str] = &["ch32v002x4x6", "ch32v006x8x6"];

fn main() {
    // ... flash mode selection ...

    let chip = CHIPS
        .iter()
        .find(|c| cfg_has(&format!("CARGO_FEATURE_{}", c.to_uppercase())))
        .expect("No chip variant selected");

    let src = format!("{manifest_dir}/memory_x/{chip}/{flash_mode}.x");
    // ...
}
```

Add each variant to the `CHIPS` array in `build.rs`, create its `memory_x/` subdirectory, and add the features to `Cargo.toml`.

## Step 5: Build and test

Build both flash modes:

```bash
cd examples/ch32/v00x

# user-flash
cargo build -p boot --features ch32v006x8x6,user-flash --no-default-features
cargo build -p app  --features ch32v006x8x6,user-flash --no-default-features

# system-flash
cargo build -p boot --features ch32v006x8x6,system-flash --no-default-features
cargo build -p app  --features ch32v006x8x6,system-flash --no-default-features
```

Then flash to hardware using `wlink` and walk through the [standard test sequence](https://github.com/OpenServoCore/tinyboot#testing) for **both** flash modes. The two modes exercise different boot-mode selection paths (see [Boot mode selection](#boot-mode-selection) above), so testing only one can miss issues in the other.

## Step 6: Update CI

Add the new series to `.github/workflows/ci.yml`. Each series has its own job with a matrix of variants. The CI runs `cargo fmt`, `cargo clippy`, and `cargo build --release` for both `system-flash` and `user-flash` modes on every variant. This ensures formatting, lint, compilation, and that the bootloader fits within the system flash region.

Add a new job following the pattern of the existing series jobs. Use the V003 job as a template for custom-target chips (nightly + `-Zbuild-std`) or the V103 job for standard-target chips (nightly with the target added).

```yaml
strategy:
  matrix:
    chip: [ch32v006x8x6, ch32v007x8x6]
```

## Step 7: Update documentation

- Add the new series to the [flash modes](flash-modes.md) table with its system/user flash sizes.
- Add an entry under [examples](examples.md) describing the new series directory.
- If the series introduces new boot-mode selection behavior, transports, or other concepts not covered by existing docs, add or update the relevant handbook pages.

## Checklist

- [ ] Feature flags added to `ch32/Cargo.toml` for each variant
- [ ] Variants added to compile-error guard in `ch32/src/lib.rs`
- [ ] HAL compiles (new `v*.rs` files if peripheral versions differ)
- [ ] `memory.x` for each variant and flash mode, with correct sizes from datasheet
- [ ] Features wired through in example `boot/Cargo.toml` and `app/Cargo.toml`
- [ ] Example `build.rs` lists all variants in `CHIPS` array
- [ ] Builds for both `system-flash` and `user-flash` modes
- [ ] CI job added for the new series in `.github/workflows/ci.yml`
- [ ] Documentation updated (flash modes table, examples page, any new concepts)
- [ ] Tested on hardware
