# tinyboot-ch32-boot

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — see the main README to get started.

CH32 platform implementation for the tinyboot bootloader. Provides storage, transport, boot metadata, and boot control backed by CH32 hardware.

## Overview

Implements the `tinyboot_core::traits::boot::Platform` trait by composing four components:

| Component       | Description                                                                          |
| --------------- | ------------------------------------------------------------------------------------ |
| `Usart`         | UART/RS-485 transport with configurable pin mapping, baud rate, and half/full duplex |
| `Storage`       | Flash read/write/erase via `embedded-storage` traits                                 |
| `BootMetaStore` | Boot state and checksum storage in reserved flash page                               |
| `BootCtl`       | Boot control (app jump, system reset, boot request detection)                        |

## Usage

```rust
tinyboot_ch32_boot::boot_version!();

use tinyboot_ch32_boot::prelude::*;

fn main() -> ! {
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        rx_pull: Pull::None,
        tx_en: None,
    });
    tinyboot_ch32_boot::run(transport, BootCtlConfig);
}
```

For chips with a hardware BOOT0 pin (e.g. CH32V103), pass a `BootCtlConfig` with the GPIO pin driving the boot mode selection circuit:

```rust
tinyboot_ch32_boot::run(transport, BootCtlConfig {
    pin: Pin::PB1,
    active_high: true,
});
```

`Storage`, `BootMetaStore`, and `BootCtl` are initialized from linker symbols automatically. The boot version is placed by `boot_version!()` in the `.tb_version` section and read at runtime via the `__tb_version` linker symbol.

See [`examples/ch32/v003/boot`](../../examples/ch32/v003/boot/) and [`examples/ch32/v103/boot`](../../examples/ch32/v103/boot/) for complete examples.

## Runtime

The bootloader includes minimal startup assembly — GP/SP init and jump to main (~20 bytes). No .data/.bss init since the system-flash bootloader uses no mutable statics. Separate assembly files per QingKe core version (`v2.S` for V003, `v3.S` for V103).

## Features

| Feature        | Description                                                          |
| -------------- | -------------------------------------------------------------------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default)                                  |
| `ch32v103c8t6` | CH32V103C8T6 chip variant                                            |
| `rt`           | Minimal runtime startup (GP/SP init, no vector table or static init) |
| `system-flash` | Bootloader runs from system flash                                    |
