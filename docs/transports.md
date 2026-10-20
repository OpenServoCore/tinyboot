# Transports

The tinyboot protocol runs over any `embedded_io::Read + Write` stream. The CH32 implementation ships a USART transport configured on two **independent** axes, both chosen by **type** in `UsartConfig`:

- **`wire`** - the **MCU's** pin arrangement and how its TX pin drives:
  - `FullDuplex { rx_pull, drive }` - separate RX and TX pins. `drive: PushPull` (the usual) or `OpenDrain` (several TX pins wired onto one line with a pull-up).
  - `HalfDuplex { drive }` - RX muxed onto the TX pin (HDSEL), one wire, parked open-drain while listening. `drive: PushPullWhileSending` (push-pull around its own writes only) or `OpenDrain` (the pull-up makes every rising edge).
- **`tx_en`** - `NoTxEn`, or `TxEn { pin, tx_level }`: a direction pin for an **external** buffer (RS-485 transceiver, DXL-style buffer). Driven to `tx_level` around writes, to the inverse while idle / reading.

Because each choice is its own type, a bootloader builds only the pin switching its board needs: no DE pin and a full-duplex wire leave no per-frame pin code at all, and a combination that would let an idle node drive a single wire (`HalfDuplex { drive: PushPull }`) does not compile.

Combining the axes gives four setups. Pick whichever matches your board.

## Setup 1: full-duplex UART (two wires)

Regular UART - separate TX and RX to the host.

```rust
Usart::new(&UsartConfig {
    wire: FullDuplex { rx_pull: Pull::None, drive: PushPull },
    tx_en: NoTxEn,
    ..
});
```

`rx_pull: Pull::Up` if the RX line can float when the host is disconnected; `Pull::None` if an external pull-up is already present.

`drive: OpenDrain` lets several devices share one host RX line without a buffer: an idle TX pin then never holds the line, and a pull-up on it owns the mark level.

## Setup 2: single-wire UART

The MCU muxes RX onto the TX pin - one wire to the host, no external buffer. Useful when both ends speak half duplex directly: probes, DXL-style servo chains, any shared single-wire bus.

```rust
Usart::new(&UsartConfig {
    // the mapping's RX pin is unused in half duplex
    wire: HalfDuplex { drive: PushPullWhileSending },
    tx_en: NoTxEn,
    ..
});
```

**Hardware requirement: the wire needs an external pull-up.** The driver parks the pin **open-drain** while idle or listening, so between frames nothing holds the line at mark except the pull-up. Without one the wire floats and the receiver sees noise and framing errors - garbage bytes the protocol has to sync past. 4.7-10 kΩ to the bus voltage is typical; go stiffer for high baud rates or long, capacitive buses.

The open-drain park is what makes the single wire **shareable**: an idle node never drives the line, so several half-duplex devices (and the host) can sit on one bus without contention. The driver also raises HDSEL before enabling the transmitter - the reverse order latches the TX output low until the first own transmission, which would clamp the whole bus from init to the first reply.

`PushPullWhileSending` flips the pin to push-pull for its own frames, so edges stay sharp at megabit rates. `OpenDrain` never drives high: it saves the per-frame switch, but the pull-up and bus capacitance set every rising edge and so the highest usable baud.

## Setup 3: full-duplex UART + external half-duplex buffer (RS-485 / DXL TTL)

The MCU runs regular full-duplex UART to a hardware transceiver (MAX485, 74LVC2G241, etc.), and `tx_en` drives the transceiver's direction pin so its output stage only drives the bus while the MCU is transmitting.

```rust
Usart::new(&UsartConfig {
    wire: FullDuplex { rx_pull: Pull::None, drive: PushPull },
    tx_en: TxEn {
        pin: Pin::PC2,
        tx_level: Level::High,   // level that puts the transceiver in TX mode
    },
    ..
});
```

`tx_level` matches the transceiver's direction-pin polarity:

- **MAX485-style** (DE active high, /RE active low, tied together): `tx_level: Level::High`.
- **Inverted driver** (e.g. some 74LVC2G241 layouts where the enable is active low): `tx_level: Level::Low`.

The pin is set to its RX level before it turns output, so the transceiver never sees a TX glitch at init.

## Setup 4: single-wire UART + external buffer

MCU half duplex **and** a direction-controlled external buffer: one MCU pin wired to both the transceiver's driver input and receiver output, with DE and /RE tied together. Pin-starved packages (e.g. the 8-pin CH32V003J4M6) use this to save a pin; most boards use setup 3.

```rust
Usart::new(&UsartConfig {
    wire: HalfDuplex { drive: PushPullWhileSending },
    tx_en: TxEn { pin: Pin::PC2, tx_level: Level::High },
    ..
});
```

The open-drain idle park applies here too, so the stub between the pin and the transceiver needs a pull-up (or a transceiver with internal biasing) to keep it at mark while the MCU listens.

## Flash cost

Bootloader size on the CH32V003 example (system-flash, 115200 baud), per setup:

| Setup                                               | Bytes |
| --------------------------------------------------- | ----- |
| 1: `FullDuplex` + `NoTxEn`                          | 1652  |
| 2: `HalfDuplex { OpenDrain }` + `NoTxEn`            | 1638  |
| 2: `HalfDuplex { PushPullWhileSending }` + `NoTxEn` | 1696  |
| 3: `FullDuplex` + `TxEn` (the example)              | 1734  |
| 4: `HalfDuplex { PushPullWhileSending }` + `TxEn`   | 1754  |

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

With `HalfDuplex`, only the TX pin is used; the RX pin of the mapping is unused (and there is no `rx_pull`).

## Matching the app side

The app's USART configuration must match the bootloader's:

- Same USART instance (e.g. USART1).
- Same pins / remap.
- Same baud rate.
- Same wire arrangement (the bootloader's `HalfDuplex` / `FullDuplex` and the app's single-wire vs two-wire UART setup).
- Same `tx_en` pin and `tx_level` (if used).

If any of these differ, the app can still run — but it won't be able to receive `Reset` or `Info` over the bus, so remote bootloader entry won't work. See the [app integration guide](app-integration.md) for the app-side wiring.

## Custom transports

The protocol is transport-agnostic — it just needs a byte-oriented duplex stream. To implement your own (USB CDC, SPI, even a radio link), implement `tinyboot_core::traits::Transport`, which is just `embedded_io::Read + Write`. See the [porting guide](porting.md) for the trait surface.
