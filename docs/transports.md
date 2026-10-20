# Transports

The tinyboot protocol runs over any `embedded_io::Read + Write` stream. The CH32 implementation ships a USART transport configured via two **independent** axes:

- **the `half-duplex` cargo feature** — controls the **MCU's** pin arrangement, at compile time (a bootloader binary serves one board, so there is no runtime switch to pay for).
  - off (default) — full duplex: separate RX and TX pins.
  - on — RX is muxed onto the TX pin (HDSEL); the MCU uses a single wire, parked open-drain while idle.
- **`tx_en`** — optional direction pin for an **external** buffer (RS-485 transceiver, etc.). Independent of the duplex mode. Driven to the configured `tx_level` around writes, to the inverse while idle / reading.

Combining these gives four useful setups. Pick whichever matches your board.

## Setup 1: full-duplex UART (two wires)

Regular UART — separate TX and RX to the host. This is the default (no feature).

```rust
Usart::new(&UsartConfig {
    tx_en: None,
    ..
});
```

`rx_pull: Pull::Up` if the RX line can float when the host is disconnected; `Pull::None` if an external pull-up is already present.

## Setup 2: single-wire UART (the `half-duplex` feature)

The MCU muxes RX onto the TX pin — one wire to the host, no external buffer. Useful when both ends speak half duplex directly: probes, DXL-style servo chains, any shared single-wire bus.

```toml
tinyboot-ch32 = { version = "...", features = ["<your-chip>", "half-duplex"] }
```

```rust
Usart::new(&UsartConfig {
    // no rx_pull: the mapping's RX pin is unused in half duplex
    tx_en: None,
    ..
});
```

**Hardware requirement: the wire needs an external pull-up.** The driver parks the pin **open-drain** while idle or listening and flips it to push-pull only around its own writes, so between frames nothing holds the line at mark except the pull-up. Without one the wire floats and the receiver sees noise and framing errors — garbage bytes the protocol has to sync past. 4.7–10 kΩ to the bus voltage is typical; go stiffer for high baud rates or long, capacitive buses.

The open-drain park is what makes the single wire **shareable**: an idle node never drives the line, so several half-duplex devices (and the host) can sit on one bus without contention. The driver also raises HDSEL before enabling the transmitter — the reverse order latches the TX output low until the first own transmission, which would clamp the whole bus from init to the first reply.

The feature costs ~130 bytes of flash over full duplex, which is why it's a compile-time switch: a full-duplex boot pays nothing (and on the CH32V003's 1920-byte system flash, has nothing to spare).

## Setup 3: full-duplex UART + external half-duplex buffer (RS-485 / DXL TTL)

The MCU runs regular full-duplex UART to a hardware transceiver (MAX485, 74LVC2G241, etc.), and `tx_en` drives the transceiver's direction pin so its output stage only drives the bus while the MCU is transmitting.

```rust
Usart::new(&UsartConfig {
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

MCU half-duplex (muxed RX/TX, the `half-duplex` feature) **and** a direction-controlled external buffer. Valid if your board puts a non-auto-direction buffer or level shifter in front of the MCU's single wire — the one pin carries both directions, and `tx_en` tells the buffer which way to point.

```rust
Usart::new(&UsartConfig {
    tx_en: Some(TxEnConfig { pin: Pin::PC2, tx_level: Level::High }),
    ..
});
```

The open-drain idle park applies in this setup too — it follows the `half-duplex` feature, independent of `tx_en` — so the stub between the pin and the buffer input needs a pull-up (or a transceiver with internal biasing) to keep the buffer's input at mark while the MCU listens.

## What `tx_en` actually does

When configured, the driver toggles the direction pin around every frame:

- Before the first byte of a write, the pin goes to `tx_level`.
- After the UART has finished transmitting (USART TC flag asserted — the driver calls `usart::flush` before releasing), the pin returns to the inverse of `tx_level`.

This keeps the transceiver in RX the rest of the time, so host bytes can reach the MCU's RX pin without contention.

## Baud rate

`BaudRate` covers the standard ladder from 9600 up to 3 Mbps: `B9600`, `B19200`, `B38400`, `B57600`, `B115200`, `B230400`, `B460800`, `B500000`, `B921600`, `B1000000`, `B1500000`, `B2000000`, `B2500000`, `B3000000`.

`UsartConfig::baud` is the build default. A rate stored in boot metadata (`tinyboot baud <rate>`, or the app's `set_baud()`, see the [app integration guide](app-integration.md)) overrides it from the next start, as long as `pclk` reaches it: the divisor for every rate is computed at build time from `pclk`, and a rate whose divisor would fall below 16 (for example anything above 500000 at 8 MHz) keeps the build default. `tinyboot baud default` clears the stored rate.

The achievable accuracy depends on `pclk`: the USART divisor is `pclk / baud`, so non-integer ratios accumulate framing error. For high baud rates you usually need to bump the core clock — e.g. the V00x example calls `rcc::init_48mhz_hsi_pll()` so PCLK = 48 MHz, which divides exactly to 3 Mbps. The CH32V003's reset-default 8 MHz PCLK is fine up to ~115200 but not for the megabit rates.

## Single-wire buses (DXL daisy chains, RS-485 segments)

On a single-wire bus where the host's TX and RX are also tied to the data line — typical for DXL chains and most RS-485 hookups — the host hears its own request frame echoed back before the device replies. The shipped `tinyboot` CLI handles this automatically by skipping any frame whose status is `Request` (devices never reply with that status). No host-side configuration needed; just match the device's baud and `tx_en` polarity.

## Pin remaps

`UsartMapping` picks the AFIO remap and selects which physical pins carry TX / RX. Available mappings are codegen'd per chip — check the generated `UsartMapping` enum in `tinyboot-ch32`, and cross-reference against the USART / AFIO sections of your chip's datasheet for the pin assignments.

With the `half-duplex` feature, only the TX pin is used; the RX pin of the mapping is unused (and `UsartConfig` has no `rx_pull` field).

## Matching the app side

The app's USART configuration must match the bootloader's:

- Same USART instance (e.g. USART1).
- Same pins / remap.
- Same baud rate.
- Same wire arrangement (the bootloader's `half-duplex` feature ↔ the app's single-wire vs two-wire UART setup).
- Same `tx_en` pin and `tx_level` (if used).

If any of these differ, the app can still run — but it won't be able to receive `Reset` or `Info` over the bus, so remote bootloader entry won't work. See the [app integration guide](app-integration.md) for the app-side wiring.

## Custom transports

The protocol is transport-agnostic — it just needs a byte-oriented duplex stream. To implement your own (USB CDC, SPI, even a radio link), implement `tinyboot_core::traits::Transport`, which is just `embedded_io::Read + Write`. See the [porting guide](porting.md) for the trait surface.
