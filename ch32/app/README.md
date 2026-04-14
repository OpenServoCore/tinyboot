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

let mut app = tinyboot_ch32_app::new_app(
    0x0800_0000, // boot_base
    4 * 1024,    // boot_size
    12 * 1024,   // app_size
    64,          // erase_size
);

// Confirm boot — transitions Validating → Idle in boot metadata
app.confirm();

// Main loop: poll for tinyboot commands
loop {
    app.poll(&mut rx, &mut tx);
}
```

## API

| Function            | Description                                                                                                                                   |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `app_version!()`    | Macro that places the crate version (from `Cargo.toml`) in the `.tb_version` linker section                                             |
| `fix_mtvec!()`      | Macro that fixes `mtvec` for apps behind a user-flash bootloader. Requires `--wrap=_setup_interrupts` linker arg. Not needed for system-flash |
| `new_app()`         | Create an `App` configured for CH32 hardware with boot/app base, sizes, and erase size                                                        |
| `App::confirm()`    | Confirm trial boot (Validating → Idle), preserving checksum in boot metadata                                                                  |
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
| `system-flash` | App paired with system-flash bootloader |
| `defmt`        | Enable defmt logging                    |

See [`examples/ch32/system-flash/app`](../examples/ch32/system-flash/app/) for a complete example.
