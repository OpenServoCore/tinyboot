# tinyboot-ch32-app

App-side tinyboot client for CH32 microcontrollers. Handles boot confirmation and responds to host commands (Info, Reset) so the CLI can query and reset the device without physical access.

## Usage

```rust
use tinyboot_ch32_app::{App, AppConfig};

// Place the app version in flash (reads from Cargo.toml)
tinyboot_ch32_app::app_version!();

let mut app = App::new(&AppConfig {
    boot_base: 0x1FFF_F000,
    boot_size: 1920,
    app_size: 16 * 1024,
    erase_size: 64,
});

// Confirm boot — transitions Validating → Idle in option bytes
app.confirm();

// Main loop: poll for tinyboot commands
loop {
    app.poll(&mut rx, &mut tx);
}
```

## API

| Function | Description |
| -------- | ----------- |
| `app_version!()` | Macro that places the crate version (from `Cargo.toml`) in the `.tinyboot_version` linker section |
| `App::new()` | Create client with flash layout configuration |
| `App::confirm()` | Confirm trial boot (Validating → Idle), preserving checksum in OB |
| `App::poll()` | Poll for and handle one tinyboot command (blocking) |
| `App::poll_async()` | Async version of `poll()` |

### Commands handled

- **Info** — responds with capacity, erase size, versions, and `mode=1` (app)
- **Reset** — resets the device; `addr=1` reboots into bootloader, `addr=0` reboots normally

All other commands receive `Status::Unsupported`.

## Features

| Feature | Description |
| ------- | ----------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default) |
| `system-flash` | App paired with system-flash bootloader |
| `defmt` | Enable defmt logging |

See [`examples/ch32/system-flash/app`](../examples/ch32/system-flash/app/) for a complete example.
