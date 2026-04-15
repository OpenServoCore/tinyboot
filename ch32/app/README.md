# tinyboot-ch32-app

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — see the main README to get started.

App-side tinyboot client for CH32 microcontrollers. Handles boot confirmation and responds to host commands (Info, Reset) so the CLI can query and reset the device without physical access.

## Usage

```rust
// Place the app version in flash (reads from Cargo.toml)
tinyboot_ch32_app::app_version!();

// For user-flash bootloaders: fix mtvec so interrupts work at non-zero addresses.
// Requires `println!("cargo:rustc-link-arg=--wrap=_setup_interrupts");` in build.rs.
// Not needed for system-flash bootloaders (app starts at 0x0).
tinyboot_ch32_app::fix_mtvec!();

// All parameters come from linker symbols — no hardcoded addresses needed.
// Pass BootCtlConfig for your chip (unit struct for V003, GPIO config for V103).
let mut app = tinyboot_ch32_app::new_app(tinyboot_ch32_app::BootCtlConfig);

// Confirm boot — transitions Validating -> Idle in boot metadata
app.confirm();

// Main loop: poll for tinyboot commands
loop {
    app.poll(&mut rx, &mut tx);
}
```

## API

| Function            | Description                                                                                                                                   |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `app_version!()`    | Macro that places the crate version (from `Cargo.toml`) in the `.tb_version` linker section                                                   |
| `fix_mtvec!()`      | Macro that fixes `mtvec` for apps behind a user-flash bootloader. Requires `--wrap=_setup_interrupts` linker arg. Not needed for system-flash |
| `new_app(config)`   | Create an `App` configured for CH32 hardware. Takes `BootCtlConfig`; reads geometry from linker symbols                                       |
| `App::confirm()`    | Confirm trial boot (Validating -> Idle), preserving checksum in boot metadata                                                                 |
| `App::poll()`       | Poll for and handle one tinyboot command (blocking)                                                                                           |
| `App::poll_async()` | Async version of `poll()`                                                                                                                     |

### Commands handled

- **Info** — responds with capacity, erase size, versions, and `mode=1` (app)
- **Reset** — resets the device; `addr=1` reboots into bootloader, `addr=0` reboots normally

All other commands receive `Status::Unsupported`.

## Features

| Feature        | Description                             |
| -------------- | --------------------------------------- |
| `ch32v003f4p6` | CH32V003F4P6 chip variant (default)     |
| `ch32v103c8t6` | CH32V103C8T6 chip variant               |
| `system-flash` | App paired with system-flash bootloader |

See [`examples/ch32/v003/app`](../../examples/ch32/v003/app/) and [`examples/ch32/v103/app`](../../examples/ch32/v103/app/) for complete examples.
