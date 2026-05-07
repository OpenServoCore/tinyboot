# Transports

The tinyboot protocol runs over any `embedded_io::Read + Write` stream. The CH32 implementation ships a USART transport configured via two **independent** axes:

- **`duplex`** — controls the **MCU's** pin arrangement.
  - `Full` — separate RX and TX pins.
  - `Half` — RX is muxed onto the TX pin; the MCU uses a single wire.
- **`tx_en`** — optional direction pin for an **external** buffer (RS-485 transceiver, etc.). Independent of `duplex`. Driven to the configured `tx_level` around writes, to the inverse while idle / reading.

Combining these gives four useful setups. Pick whichever matches your board.

## Setup 1: full-duplex UART (two wires)

Regular UART — separate TX and RX to the host.

```rust
Usart::new(&UsartConfig {
    duplex: Duplex::Full,
    tx_en: None,
    ..
});
```

`rx_pull: Pull::Up` if the RX line can float when the host is disconnected; `Pull::None` if an external pull-up is already present.

## Setup 2: single-wire UART (MCU-level half duplex)

The MCU muxes RX onto the TX pin — one wire to the host, no external buffer. Useful when both ends speak half duplex directly (some probes, some DXL servo chains where the MCU is the sole driver on its segment).

```rust
Usart::new(&UsartConfig {
    duplex: Duplex::Half,
    tx_en: None,
    ..
});
```

## Setup 3: full-duplex UART + external half-duplex buffer (RS-485 / DXL TTL)

This is the OpenServoCore hardware style: the MCU runs regular full-duplex UART to a hardware transceiver (MAX485, 74LVC2G241, etc.), and `tx_en` drives the transceiver's direction pin so its output stage only drives the bus while the MCU is transmitting.

```rust
Usart::new(&UsartConfig {
    duplex: Duplex::Full,
    tx_en: Some(TxEnConfig {
        pin: Pin::PC2,
        tx_level: Level::High,   // level that puts the transceiver in TX mode
    }),
    ..
});
```

`tx_level` matches the transceiver's direction-pin polarity:

- **MAX485-style** (DE active high, /RE active low, tied together): `tx_level: Level::High`.
- **Inverted driver** (e.g. some 74LVC2G241 layouts where the enable is active low): `tx_level: Level::Low`.

## Setup 4: single-wire UART + external buffer

MCU half-duplex (muxed RX/TX) **and** a direction-controlled external buffer. Valid if your board puts a buffer in front of the MCU's single wire and still needs direction switching.

```rust
Usart::new(&UsartConfig {
    duplex: Duplex::Half,
    tx_en: Some(TxEnConfig { pin: Pin::PC2, tx_level: Level::High }),
    ..
});
```

## What `tx_en` actually does

When configured, the driver toggles the direction pin around every frame:

- Before the first byte of a write, the pin goes to `tx_level`.
- After the UART has finished transmitting (USART TC flag asserted — the driver calls `usart::flush` before releasing), the pin returns to the inverse of `tx_level`.

This keeps the transceiver in RX the rest of the time, so host bytes can reach the MCU's RX pin without contention.

## Baud rate

`BaudRate` covers the standard ladder from 9600 up to 3 Mbps: `B9600`, `B19200`, `B38400`, `B57600`, `B115200`, `B230400`, `B460800`, `B500000`, `B921600`, `B1000000`, `B1500000`, `B2000000`, `B2500000`, `B3000000`.

The achievable accuracy depends on `pclk`: the USART divisor is `pclk / baud`, so non-integer ratios accumulate framing error. For high baud rates you usually need to bump the core clock — e.g. the V00x example calls `rcc::init_48mhz_hsi_pll()` so PCLK = 48 MHz, which divides exactly to 3 Mbps. The CH32V003's reset-default 8 MHz PCLK is fine up to ~115200 but not for the megabit rates.

## Single-wire buses (DXL daisy chains, RS-485 segments)

On a single-wire bus where the host's TX and RX are also tied to the data line — typical for DXL chains and most RS-485 hookups — the host hears its own request frame echoed back before the device replies. The shipped `tinyboot` CLI handles this automatically by skipping any frame whose status is `Request` (devices never reply with that status). No host-side configuration needed; just match the device's baud and `tx_en` polarity.

## Pin remaps

`UsartMapping` picks the AFIO remap and selects which physical pins carry TX / RX. Available mappings are codegen'd per chip — check the generated `UsartMapping` enum in `tinyboot-ch32`, and cross-reference against the USART / AFIO sections of your chip's datasheet for the pin assignments.

In `Duplex::Half`, only the TX pin is used; the RX pin of the mapping is unused.

## Matching the app side

The app's USART configuration must match the bootloader's:

- Same USART instance (e.g. USART1).
- Same pins / remap.
- Same baud rate.
- Same `duplex` mode.
- Same `tx_en` pin and `tx_level` (if used).

If any of these differ, the app can still run — but it won't be able to receive `Reset` or `Info` over the bus, so remote bootloader entry won't work. See the [app integration guide](app-integration.md) for the app-side wiring.

## Custom transports

The protocol is transport-agnostic — it just needs a byte-oriented duplex stream. To implement your own (USB CDC, SPI, even a radio link), implement `tinyboot_core::traits::Transport`, which is just `embedded_io::Read + Write`. See the [porting guide](porting.md) for the trait surface.
