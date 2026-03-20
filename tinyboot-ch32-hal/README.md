# tinyboot-ch32-hal

Minimal hardware abstraction layer for tinyboot on CH32 microcontrollers. This is not a general-purpose HAL — it provides only what the bootloader needs, optimized for code size.

## Modules

| Module | Description |
| ------ | ----------- |
| `flash` | Flash read/write/erase, option byte access |
| `gpio`  | Pin configuration (input, output, alternate function) |
| `usart` | UART transmit/receive with embedded-io traits |
| `rcc`   | Clock configuration |
| `afio`  | Alternate function I/O and pin remapping |
| `pfic`  | Interrupt controller and system reset |

When `system-flash` is disabled, the `boot_request` module is also available for RAM-based boot request signaling.

## Code generation

`build.rs` reads chip metadata from `ch32-metapac` and generates:

- **`Pin` enum** — all GPIO pins with bit-packed discriminants for table-free port/pin extraction
- **`UsartMapping` enum** — all USART remap configurations with TX/RX pin and register accessors

This keeps the source chip-agnostic while producing zero-overhead accessors.

## Features

| Feature | Description |
| ------- | ----------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default) |
| `system-flash` | Bootloader runs from system flash (uses BOOT_MODE register for boot requests) |

## Notes

- `critical-section` is included but implemented as a no-op — the bootloader runs with interrupts disabled.
- All modules use PAC register access directly for minimal code size.
