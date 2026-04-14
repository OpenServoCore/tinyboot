# tinyboot-ch32-hal

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — see the main README to get started.

Minimal hardware abstraction layer for tinyboot on CH32 microcontrollers. This is not a general-purpose HAL — it provides only what the bootloader needs, optimized for code size.

## Modules

| Module         | Description                                           |
| -------------- | ----------------------------------------------------- |
| `flash`        | Flash page erase/write, boot metadata address         |
| `gpio`         | Pin configuration (input, output, alternate function) |
| `usart`        | UART transmit/receive with embedded-io traits         |
| `rcc`          | Clock configuration and peripheral reset              |
| `afio`         | Alternate function I/O and pin remapping              |
| `pfic`         | Interrupt controller and system reset                 |
| `iwdg`         | Independent watchdog timer feed                       |
| `boot_request` | RAM-based boot request signaling (user-flash only)    |

## Code generation

`build.rs` reads chip metadata from `ch32-metapac` and generates:

- **`Pin` enum** — all GPIO pins with bit-packed discriminants for table-free port/pin extraction
- **`UsartMapping` enum** — all USART remap configurations with TX/RX pin and register accessors

This keeps the source chip-agnostic while producing zero-overhead accessors.

## Features

| Feature        | Description                                                                   |
| -------------- | ----------------------------------------------------------------------------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default, for rust-analyzer; CI uses explicit chip) |
| `system-flash` | Bootloader runs from system flash (uses BOOT_MODE register for boot requests) |

## Notes

- All modules use PAC register access directly for minimal code size.
- The `default` feature (`ch32v003f4p6`) exists so rust-analyzer can analyze the ch32 workspace without explicit feature flags. Downstream crates use `default-features = false`.
