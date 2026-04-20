# tinyboot docs

Documentation for using, integrating, and extending tinyboot.

If you're new here, start with the [top-level README](https://github.com/OpenServoCore/tinyboot#quick-start-ch32v003) quick start, then come back for deeper topics.

## Tutorial

- [Getting Started](getting-started.md) — toolchain, tools, and your first successful flash

## Guides

- [Flash modes: system-flash vs user-flash](flash-modes.md)
- [Transports: UART, RS-485, DXL TTL](transports.md)
- [GPIO-controlled boot mode selection](boot-ctl.md) — BOOT0 circuits for chips with hardware boot pins
- [App integration](app-integration.md) — wire the tinyboot app side into your firmware
- [Remote firmware updates](remote-updates.md) — the end-to-end OTA flow
- [Building your bootloader from an example](examples.md)

## Reference

- [Porting to a new MCU family](porting.md)
- [Design notes](design.md) — motivation, the 1920-byte budget, unsafe policy
- [Protocol reference](https://github.com/OpenServoCore/tinyboot/tree/main/lib/protocol) — wire format, frames, commands
- [Boot state machine](https://github.com/OpenServoCore/tinyboot/tree/main/lib/core) — state transitions, metadata layout
- [CLI reference](https://github.com/OpenServoCore/tinyboot/tree/main/cli)
- [`tinyboot-ch32` reference](https://github.com/OpenServoCore/tinyboot/tree/main/ch32)

## Troubleshooting

- [Troubleshooting guide](troubleshooting.md) — symptoms, likely causes, fixes

## Contributing

- [Contributing](contributing.md) — dev setup, tests, hardware validation
