# tinyboot-ch32

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — start with the [top-level README](https://github.com/OpenServoCore/tinyboot#quick-start-ch32v003) and the [handbook](https://openservocore.github.io/tinyboot/).

CH32 HAL and tinyboot platform for CH32V003, CH32V00x (V002 / V004 / V005 / V006 / V007), and CH32V103. Exposes a bootloader-side entry point ([`boot`]) and an app-side client ([`app`]) built on a small in-crate HAL ([`hal`]).

## Installation

```toml
[dependencies]
tinyboot-ch32 = { version = "0.6", default-features = false, features = ["ch32v006x8x6", "system-flash"] }
tinyboot-ch32-rt = "0.4"  # optional, bootloader-only
```

## Modules

| Module     | For                  | What it provides                                                                                 |
| ---------- | -------------------- | ------------------------------------------------------------------------------------------------ |
| `boot`     | Bootloader binaries  | `run()`, `BootCtl`, `Addressable` / `Unaddressed`, `chip_uid`, USART transport (`Usart`, `UsartConfig`, `BaudRate`, `FullDuplex` / `HalfDuplex`, `TxEn` / `NoTxEn`) |
| `app`      | Application binaries | `new_app()`, `new_addressable_app()`, `App`, `BootCtl`, the `tinyboot_core::app` types                                    |
| `hal`      | Both                 | `flash`, `gpio`, `usart`, `afio`, `rcc`, `pfic`, `iwdg`; auto-generated `Pin` and `UsartMapping` |
| `platform` | (internal)           | `tinyboot_core::traits` impls for Storage, Transport, BootCtl, BootMetaStore                     |

## Minimal bootloader

```rust
use panic_halt as _;
use tinyboot_ch32_rt as _;

tinyboot_ch32::boot::boot_version!();

use tinyboot_ch32::boot::prelude::*;

#[unsafe(export_name = "main")]
fn main() -> ! {
    let transport = Usart::new(&UsartConfig {
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        wire: FullDuplex { rx_pull: Pull::None, drive: PushPull },
        tx_en: NoTxEn,
    });
    tinyboot_ch32::boot::run(transport, BootCtl::new(), Unaddressed);
}
```

`Unaddressed` answers every frame, for a device that has the bus to itself. With several devices on one bus, pass `Addressable::new(chip_uid)` instead: the bootloader then stays silent until a host selects it by the chip's unique id (see the [shared bus guide](https://openservocore.github.io/tinyboot/shared-bus.html)).

`Storage` and `BootMetaStore` are initialized from linker symbols automatically. `boot_version!()` ends the image with its tail: a compatibility key and the crate's `Cargo.toml` version. `boot_version!(compat = "name")` sets the key, and the bootloader then accepts only app images built with the same `app_version!(compat = "name")` (see [app integration](https://openservocore.github.io/tinyboot/app-integration.html)).

`boot::run` reads frames with up to 52 payload bytes (a whole frame fits one 64-byte USB packet); `boot::run_with_payload::<N>` takes `N` bytes per Write (at least 18, not limited to a flash page) for faster updates, at `N` bytes of RAM. Hosts read the capacity from Info.

For configuring RS-485 half-duplex, DXL TTL, or alternate pin remaps, see the [transports guide](https://openservocore.github.io/tinyboot/transports.html). For CH32V103 in system-flash mode, `BootCtl::new` takes additional arguments for the external BOOT_CTL circuit — see [boot-ctl](https://openservocore.github.io/tinyboot/boot-ctl.html).

## Minimal app

```rust
tinyboot_ch32::app::app_version!();

let mut app = tinyboot_ch32::app::new_app(tinyboot_ch32::app::BootCtl::new());
app.confirm();

loop {
    app.poll(&mut rx, &mut tx);
}
```

`app::poll` handles `Info` (responds with capacity, erase size, versions, `mode = 1`, the stored boot baud and the bootloader's compatibility key) and `Reset` (resets the device; `ResetFlags::BOOTLOADER` reboots into the bootloader, otherwise into the app). All other commands return `Status::Unsupported`. On a shared bus, `app::new_addressable_app(ctl, wait_bytes)` also handles `Select` and stays silent until selected.

See the [app integration guide](https://openservocore.github.io/tinyboot/app-integration.html) for a complete app including `embedded_io` transport wrapping and peripheral ownership.

## Features

| Feature                                                            | Description                                  |
| ------------------------------------------------------------------ | -------------------------------------------- |
| `ch32v003f4p6` / `a4m6` / `f4u6` / `j4m6`                          | CH32V003 chip variants                       |
| `ch32v002x4x6` / `v004x6x1` / `v005x6x6` / `v006x8x6` / `v007x8x6` | CH32V00x chip variants                       |
| `ch32v103c6t6` / `c8t6` / `c8u6` / `r8t6`                          | CH32V103 chip variants                       |
| `system-flash`                                                     | Build for the system-flash bootloader region |

See the [flash modes guide](https://openservocore.github.io/tinyboot/flash-modes.html) for when to pick `system-flash` vs user-flash.

Complete boot + app examples live in [`examples/ch32/v003`](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v003), [`examples/ch32/v00x`](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v00x), and [`examples/ch32/v103`](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v103).

## Linker scripts

The crate ships `tb-run-mode.x`, which reserves a NOLOAD 4-byte magic word at the start of RAM for run-mode persistence in builds that use it. Boot and app binaries add it to their link args alongside `tb-boot.x` / `tb-app.x`:

```sh
cargo:rustc-link-arg=-Ttb-boot.x
cargo:rustc-link-arg=-Ttb-run-mode.x
```

The core linker scripts (`tb-boot.x`, `tb-app.x`) are shipped by `tinyboot-core`. For the linker region contract every `memory.x` must satisfy, see the [porting guide](https://openservocore.github.io/tinyboot/porting.html#linker-region-contract).
