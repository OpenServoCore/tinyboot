# tinyboot-ch32-boot

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — see the main README to get started.

CH32 platform implementation for the tinyboot bootloader. Provides storage, transport, boot metadata, and boot control backed by CH32 hardware.

## Overview

Implements the `tinyboot::traits::boot::Platform` trait by composing four components:

| Component       | Description                                                                          |
| --------------- | ------------------------------------------------------------------------------------ |
| `Usart`         | UART/RS-485 transport with configurable pin mapping, baud rate, and half/full duplex |
| `Storage`       | Flash read/write/erase via `embedded-storage` traits                                 |
| `BootMetaStore` | Option-byte-based boot state and checksum storage                                    |
| `BootCtl`       | Boot control (app jump, system reset, boot request detection)                        |

## Usage

```rust
use tinyboot_ch32_boot::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Duplex,
    Platform, Pull, Storage, StorageConfig, Usart, UsartConfig, UsartMapping,
    pkg_version,
};

let transport = Usart::new(&UsartConfig {
    duplex: Duplex::Full,
    baud: BaudRate::B115200,
    pclk: 8_000_000,
    mapping: UsartMapping::Usart1Remap0,
    rx_pull: Pull::None,
    tx_en: None,
});

let storage = Storage::new(StorageConfig {
    app_base: 0x0800_0000,
    app_size: 16 * 1024,
});

let boot_meta = BootMetaStore::default();
let ctl = BootCtl::new(BootCtlConfig {});

let platform = Platform::new(transport, storage, boot_meta, ctl);
tinyboot_ch32_boot::run(platform);
```

The boot version is read at runtime from the `__tinyboot_version` linker symbol (placed by `boot_version!()` in the `.tinyboot_version` section).

See [`examples/ch32/system-flash`](../examples/ch32/system-flash/) for a complete bootloader example.

## Runtime

The bootloader includes two startup assembly files, selected by the `defmt` feature:

- **`v2.S`** (default) — minimal startup (GP/SP init + jump to main, ~20 bytes). Omits .data/.bss init since the system-flash bootloader uses no mutable statics.
- **`v2_full.S`** (`defmt` enabled) — full startup with .data copy and .bss zeroing, required for defmt-rtt and safe app→bootloader resets.

## Features

| Feature        | Description                                                          |
| -------------- | -------------------------------------------------------------------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default)                                  |
| `rt`           | Minimal runtime startup (GP/SP init, no vector table or static init) |
| `system-flash` | Bootloader runs from system flash                                    |
| `defmt`        | Enable defmt logging                                                 |
