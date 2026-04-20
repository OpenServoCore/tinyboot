# tinyboot-ch32

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — see the main README to get started.

CH32 HAL and tinyboot platform for CH32V003, CH32V00x (V002/V004/V005/V006/V007), and CH32V103. Exposes a bootloader-side entry point ([`boot`]) and an app-side client ([`app`]) built on a small in-crate HAL ([`hal`]).

## Installation

As of v0.4.0, `tinyboot-ch32` is **consumed from git**, not crates.io. It depends on [`ch32-metapac`](https://github.com/ch32-rs/ch32-metapac) as a git-only dependency for CH32V00x flash support, which crates.io does not allow. Add it to your `Cargo.toml` like so:

```toml
[dependencies]
tinyboot-ch32 = { git = "https://github.com/OpenServoCore/tinyboot", tag = "v0.4.0", default-features = false, features = ["ch32v006x8x6", "system-flash"] }
tinyboot-ch32-rt = "0.4"  # optional, bootloader-only; on crates.io
```

`tinyboot-core`, `tinyboot-protocol`, `tinyboot-ch32-rt`, and the `tinyboot` CLI are all published to crates.io. Only `tinyboot-ch32` requires the git path until the upstream flash driver lands in a `ch32-metapac` release.

## Modules

| Module     | For                  | What it provides                                                                                 |
| ---------- | -------------------- | ------------------------------------------------------------------------------------------------ |
| `boot`     | Bootloader binaries  | `run()`, `BootCtl`, USART transport (`Usart`, `UsartConfig`, `BaudRate`, `Duplex`, `TxEnConfig`) |
| `app`      | Application binaries | `new_app()`, `App`, `BootCtl`, the `tinyboot_core::app` types                                    |
| `hal`      | Both                 | `flash`, `gpio`, `usart`, `afio`, `rcc`, `pfic`, `iwdg`; auto-generated `Pin` and `UsartMapping` |
| `platform` | (internal)           | `tinyboot_core::traits` impls for Storage, Transport, BootCtl, BootMetaStore                     |

## Bootloader example

```rust
use panic_halt as _;
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!();

use tinyboot_ch32::boot::prelude::*;

#[unsafe(export_name = "main")]
fn main() -> ! {
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        rx_pull: Pull::None,
        tx_en: None,
    });
    tinyboot_ch32::boot::run(transport, BootCtl::new());
}
```

`Storage` and `BootMetaStore` are initialized from linker symbols automatically. `boot_version!()` places the crate's `Cargo.toml` version into the `.tb_version` section; the core reads it via `__tb_version`.

For CH32V103 in `system-flash` mode, `BootCtl::new` takes a GPIO pin driving the external BOOT0 circuit, the level that selects system flash, and a reset-delay cycle count (RC settle time):

```rust
BootCtl::new(Pin::PB1, Level::High, 8000)
```

For RS-485 half-duplex with a DE/RE pin:

```rust
Usart::new(&UsartConfig {
    duplex: Duplex::Half,
    tx_en: Some(TxEnConfig { pin: Pin::PC2, tx_level: Level::High }),
    ..
})
```

## App example

```rust
tinyboot_ch32::app::app_version!();

let mut app = tinyboot_ch32::app::new_app(tinyboot_ch32::app::BootCtl::new());
app.confirm();

loop {
    app.poll(&mut rx, &mut tx);
}
```

`app::poll` handles Info and Reset:

- **Info** — responds with capacity, erase size, versions, and `mode=1`.
- **Reset** — resets the device; `addr=1` reboots into the bootloader, `addr=0` reboots into the app.

All other commands return `Status::Unsupported`.

For CH32V103 `system-flash` apps, pass the same `BootCtl::new(pin, level, delay)` as the bootloader. Apps on V003 or in `user-flash` mode use the unit-arg form `BootCtl::new()`.

## Features

| Feature                                                            | Description                                  |
| ------------------------------------------------------------------ | -------------------------------------------- |
| `ch32v003f4p6` / `a4m6` / `f4u6` / `j4m6`                          | CH32V003 chip variants                       |
| `ch32v002x4x6` / `v004x6x1` / `v005x6x6` / `v006x8x6` / `v007x8x6` | CH32V00x chip variants                       |
| `ch32v103c6t6` / `c8t6` / `c8u6` / `r8t6`                          | CH32V103 chip variants                       |
| `system-flash`                                                     | Build for the system-flash bootloader region |

Complete boot + app examples live in [`examples/ch32/v003`](../examples/ch32/v003/), [`examples/ch32/v00x`](../examples/ch32/v00x/), and [`examples/ch32/v103`](../examples/ch32/v103/).

## Linker scripts

The crate ships `tb-run-mode.x`, which reserves a NOLOAD 4-byte magic word at the start of RAM for run-mode persistence in builds that use it. Boot and app binaries add it to their link args alongside `tb-boot.x` / `tb-app.x`:

```sh
cargo:rustc-link-arg=-Ttb-boot.x
cargo:rustc-link-arg=-Ttb-run-mode.x
```

The core linker scripts (`tb-boot.x`, `tb-app.x`) are shipped by `tinyboot-core`.
