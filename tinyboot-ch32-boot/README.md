# tinyboot-ch32-boot

CH32 platform implementation for the tinyboot bootloader. Provides storage, transport, boot metadata, and boot control backed by CH32 hardware.

## Overview

Implements the `tinyboot::traits::boot::Platform` trait by composing four components:

| Component | Description |
| --------- | ----------- |
| `Usart` | UART/RS-485 transport with configurable pin mapping, baud rate, and half/full duplex |
| `Storage` | Flash read/write/erase via `embedded-storage` traits |
| `BootMetaStore` | Option-byte-based boot state and checksum storage |
| `BootCtl` | Boot control (app jump, system reset, boot request detection) |

## Usage

```rust
use tinyboot_ch32_boot::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Core, Duplex,
    Platform, Pull, Storage, StorageConfig, Usart, UsartConfig, UsartMapping,
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
    boot_base: 0x1FFF_F000,
    boot_size: 1920,
    app_base: 0x0800_0000,
    app_size: 16 * 1024,
});

let platform = Platform::new(transport, storage, BootMetaStore, BootCtl::new(BootCtlConfig {}));
Core::new(platform).run();
```

See [`examples/ch32/system-flash`](../examples/ch32/system-flash/) for a complete bootloader example.

## Features

| Feature | Description |
| ------- | ----------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default) |
| `rt` | Runtime startup code (reset vector, stack init) |
| `system-flash` | Bootloader runs from system flash |
| `defmt` | Enable defmt logging |
