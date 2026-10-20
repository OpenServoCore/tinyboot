# App integration

The tinyboot bootloader is only half the story — to support remote firmware updates, your app has to cooperate with it. This guide walks through what the app needs to do and shows a minimal integration.

## What the app is responsible for

1. **Declare its version** so the host can see it via `tinyboot info`.
2. **Confirm successful boot** so the bootloader stops retrying.
3. **Poll the transport** for `Info` and `Reset` requests.

That's it. The bootloader handles everything else — flashing, verification, state transitions, trial boot.

## Minimal app

```rust
#![no_std]
#![no_main]

// Embed version into the app's binary so tinyboot can find it.
tinyboot_ch32::app::app_version!();

#[qingke_rt::entry]
fn main() -> ! {
    // Your usual peripheral setup.
    let p = ch32_hal::init(Default::default());

    // UART wired the same way as the bootloader's.
    let uart = Uart::new_blocking::<0>(p.USART1, p.PD6, p.PD5, uart_config).unwrap();
    let (tx, rx) = uart.split();

    // Adapt your tx/rx to embedded_io::Read + Write (see examples/ for a sample).
    let mut rx = /* wrap rx */;
    let mut tx = /* wrap tx */;

    // Create the app handle and confirm that this boot succeeded.
    let mut app = tinyboot_ch32::app::new_app(tinyboot_ch32::app::BootCtl::new());
    app.confirm();

    loop {
        // Your app's real work goes here, alongside polling.
        app.poll(&mut rx, &mut tx);
    }
}
```

See [`examples/ch32/v003/app/`](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v003/app) for a complete example including a `transport.rs` module that wraps ch32-hal's `Uart` in the `embedded_io` traits plus optional RS-485 DE-pin handling.

## `app::confirm()` — trial boot handshake

The bootloader tracks newly-flashed firmware in a trial state. Every boot in `Validating` state consumes one trial; when trials run out, the bootloader assumes the app is broken and takes over on the next reset.

`app::confirm()` tells the bootloader the new firmware is alive. Call it **after** your app is initialized to the point where you're confident it's running correctly — early enough that it always runs on a successful boot, but late enough to catch major initialization failures.

Once called, the app is considered confirmed and will boot normally on every subsequent reset (until the next firmware update starts the cycle again).

If `confirm()` is never reached (panic, watchdog, init deadlock), the trials get consumed across resets and the bootloader eventually takes back control.

## `app::poll()` — handling bootloader commands

`poll()` reads a single frame from your transport and handles it. In the app, two commands do something; the rest are rejected with `Status::Unsupported`:

| Command | Behavior in app                                                                                                                                                  |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Info`  | Responds with capacity, erase size, boot + app versions, `mode = 1` (app mode).                                                                                  |
| `Reset` | Resets the device. The `BOOTLOADER` flag reboots into the bootloader, otherwise into the app. The `BAUD` flag first stores addr byte 0 as the bootloader's baud. |

This is enough for the host CLI to do `tinyboot info` and `tinyboot reset --bootloader` while the app is running, which is how remote updates get kicked off — see the [remote updates guide](remote-updates.md).

Because `poll()` is blocking on a read, a typical app runs it in a dedicated task or a loop iteration alongside its other work. For timing-sensitive apps, consider running the transport on an interrupt-driven reader and feeding `poll()` asynchronously; `poll()` itself is CPU-cheap.

## `baud()` and `set_baud()` - one rate for both modes

The bootloader's baud lives in boot metadata, next to the trial state. `tinyboot baud <rate>` stores it over the wire (a `Reset` with the `BAUD` flag, in either mode), and the app can store it with `app.set_baud(BaudRate::B1000000)`, for example when its own protocol changes the bus rate. The bootloader picks it up from its next start; a rate its clock cannot reach falls back to its `UsartConfig::baud`.

`app.baud()` returns the stored rate, `None` when nothing is stored. An app that brings its UART up at that rate stays reachable at one baud whichever mode the device is in; the examples do this:

```rust
let mut app = tinyboot_ch32::app::new_app(tinyboot_ch32::app::BootCtl::new());
uart_config.baudrate = app.baud().map_or(115200, |b| b.hz());
```

`set_baud()` erases and rewrites the metadata page (skipped when the rate is already stored), in a critical section like `confirm()`.

## Compatibility key - only the right image installs

`app_version!()` and `boot_version!()` end each image with a 6-byte tail: a 32-bit compatibility key, then the version. Give the bootloader a key and it accepts only images that carry the same one:

```rust
// bootloader
tinyboot_ch32::boot::boot_version!(compat = "acme-widget-rev-b");
// app
tinyboot_ch32::app::app_version!(compat = "acme-widget-rev-b");
```

`compat = "name"` hashes the name at compile time (32-bit FNV-1a); `compat_key = 0x1234_5678` takes a raw value. tinyboot only compares keys, so encode whatever has to match: product, board revision, pinout. `Verify` refuses an image with another key, or none, with `Status::Incompatible` and leaves the device in its bootloader waiting for the right image; a mistaken image never starts. A bootloader built without a key accepts any image and carries no check.

## UART sharing notes

The bootloader and app normally share the same USART. A few gotchas:

- **Matching config** — the app's baud rate, pins, and DE polarity must match the bootloader's. See [transports.md](transports.md).
- **DE pin polarity** — on boards where RS-485 transceiver contention is possible (e.g. some OpenServoCore V006 layouts), use a `tx_level` that leaves the bus driver **disabled** when idle, so the host's TX line can reach MCU_RX.
- **Half-duplex flush** — when sending multi-byte responses on half-duplex, make sure your `embedded_io::Write` implementation flushes the UART before releasing DE.

## Passing peripherals to `poll()`

`poll()` takes your transport as split rx/tx types implementing `embedded_io::Read` and `embedded_io::Write`. This lets your app keep full ownership of peripheral initialization — tinyboot doesn't take over USART registers, and you can layer extra features (logging, RTU framing) on top of the same UART if you adapt them correctly.

## BootCtl in the app

`BootCtl::new()` takes the same arguments in the app as it does in the bootloader — for CH32V003 / V00x that's `BootCtl::new()`, for CH32V103 in system-flash mode it's `BootCtl::new(pin, level, delay)`. The app needs this so that `Reset` with the `BOOTLOADER` flag can set the run-mode marker before resetting.
