# Building your bootloader from an example

The `examples/` directory holds complete, buildable boot + app projects for each supported chip family. They double as CI test targets, which is why they look more structured than a typical example — but they're also the fastest way to start your own project: copy the one that matches your chip and trim it down.

This page walks through what's in an example, what you need to change, and what you can delete.

## What's in `examples/`

```
examples/ch32/
  v003/      CH32V003 (1920 B system flash, 16 KB user flash)
  v00x/      CH32V00x (V002 / V004 / V005 / V006 / V007)
  v103/      CH32V103 (needs BOOT_CTL circuit for system-flash mode)
```

Each chip directory is a Cargo workspace with two members:

```
v003/
  Cargo.toml        workspace
  boot/             bootloader binary
    Cargo.toml
    build.rs        picks the right memory.x for the selected flash mode
    memory_x/
      system-flash.x
      user-flash.x
    src/main.rs
  app/              demo app binary
    Cargo.toml
    build.rs
    memory_x/
    src/main.rs
  rust-toolchain.toml
  riscv32ec-unknown-none-elf.json   (V003 / V00x only — custom target)
```

## Why are there so many feature flags?

The example workspaces are built across a CI matrix: multiple chip variants × system-flash / user-flash modes. Features like `ch32v003f4p6`, `system-flash`, `user-flash` exist so CI can re-use the same source tree for every combination.

**For your own project, you don't need any of that.** Pick one chip variant and one flash mode; pin them as defaults in your boot crate's `Cargo.toml`; delete the rest.

## Starting your own project from an example

1. Copy the example that matches your chip (e.g. `examples/ch32/v003/`) to a new directory.
2. In the `boot/Cargo.toml`, remove the extra chip-variant features you don't need. Leave one, set as the default.
3. Pick a flash mode. Delete the `memory_x/` file you don't need, and simplify `build.rs` to just copy the remaining one.
4. In `src/main.rs`, change the UART config (pins, baud, duplex, tx_en) to match your board.
5. Do the same for `app/` — match the UART config, adjust your pins.

That gives you a minimal, single-purpose workspace with none of the CI scaffolding.

## Using tinyboot-ch32 from crates.io

The examples depend on `tinyboot-ch32` via a path reference (`path = "../../../../ch32"`) because they live in this repo. For an external project, switch to a git dependency:

```toml
[dependencies]
tinyboot-ch32 = { git = "https://github.com/OpenServoCore/tinyboot", tag = "v0.4.0", default-features = false, features = ["ch32v003f4p6", "system-flash"] }
tinyboot-ch32-rt = "0.4"
```

`tinyboot-ch32` is git-only until upstream `ch32-metapac` publishes the flash driver it depends on. See the [`tinyboot-ch32` README](https://github.com/OpenServoCore/tinyboot/tree/main/ch32#installation) for details.

## `memory.x` and the linker region contract

Every tinyboot `memory.x` defines the same five regions: `CODE`, `BOOT`, `APP`, `META`, `RAM`. The linker scripts shipped by `tinyboot-core` derive all the chip-agnostic symbols (`__tb_*`) from those regions — you don't need to poke at magic addresses. See the [porting guide](porting.md#linker-region-contract) for the contract.

If you change chip variants (e.g. V003F4P6 → V003A4M6), the defaults in `memory.x` are usually fine — you only need to adjust if your part has non-standard RAM / flash sizes.

## `build.rs` and linker scripts

The build.rs job is to make your `memory.x` discoverable to the linker and to pull in the tinyboot linker fragments via `-T` flags. A minimal single-mode bootloader `build.rs` looks like this:

```rust
fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::copy("memory.x", format!("{out_dir}/memory.x")).unwrap();

    println!("cargo:rustc-link-search={out_dir}");
    println!("cargo:rerun-if-changed=memory.x");

    println!("cargo:rustc-link-arg=-Ttb-boot.x");
    println!("cargo:rustc-link-arg=-Ttb-run-mode.x");
}
```

For the **app** crate, swap `-Ttb-boot.x` for `-Ttb-app.x`. The rest is identical.

The example `build.rs` files in this repo look more involved because they read `CARGO_FEATURE_SYSTEM_FLASH` / `CARGO_FEATURE_USER_FLASH` to pick between `memory_x/system-flash.x` and `memory_x/user-flash.x` — that's only needed if you want a single crate that builds both modes. A user project typically picks one mode and keeps a flat `memory.x` at the crate root.

### Linker scripts

| Script              | Shipped by         | For           | When to include                                                              |
| ------------------- | ------------------ | ------------- | ---------------------------------------------------------------------------- |
| `memory.x`          | you                | Both          | Always. Defines the five regions (`CODE`, `BOOT`, `APP`, `META`, `RAM`).     |
| `tb-boot.x`         | `tinyboot-core`    | Bootloader    | Always, in the bootloader binary. Derives `__tb_*` symbols from `memory.x` and places the boot version tag. |
| `tb-app.x`          | `tinyboot-core`    | App           | Always, in the app binary. Derives `__tb_*` symbols and places the app version tag last in flash. |
| `tb-run-mode.x`     | `tinyboot-ch32`    | Both          | When the platform uses a RAM magic word for run-mode persistence (the default on V003 / V00x / V103). Reserves `__tb_run_mode` at `ORIGIN(RAM) + LENGTH(RAM)` — your `memory.x` must size `RAM` to leave 4 bytes free at the top (`LENGTH = <ram_size> - 4`). |
| `split-sysflash.x`  | `tinyboot-ch32`    | Bootloader    | Only on V103 in system-flash mode. Places `.text2` overflow code into the secondary system-flash region (`CODE2` / `BOOT2`). See [flash modes](flash-modes.md) for the V103 split layout. |

All `tb-*.x` scripts are added via `cargo:rustc-link-arg=-T<name>.x` in `build.rs`. The shipping crates put them on the linker search path automatically as part of their own build scripts, so you only need the `-T` flags.
