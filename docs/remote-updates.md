# Remote firmware updates

Once the bootloader is in system flash and your app calls `poll()` + `confirm()`, you can update the firmware over the same UART / RS-485 bus the device uses for normal operation. No probe, no reset button, no shell to open.

This guide covers the end-to-end flow.

## The short version

```sh
# Ask the running app to reboot into the bootloader.
tinyboot reset --bootloader

# Flash the new app and jump straight into it after verify.
tinyboot flash new-firmware.elf --reset
```

That's it. Everything below is what's happening under the hood.

## The lifecycle

```
power-on
   │
   ▼
bootloader starts                   META.state
   │                                   │
   ├─ META.state == Idle ──► validate app image ──► hand off to app
   │                                     │
   │                                     └─► (CRC mismatch) stay in bootloader
   │
   ├─ META.state == Validating ──► consume 1 trial, boot app
   │                                     │
   │                                     └─► (no trials left) stay in bootloader
   │
   └─ META.state == Updating ──► stay in bootloader (prior update interrupted)

app starts
   │
   ├─ app::confirm() ──► META.state → Idle (keeps current checksum)
   │
   └─ app::poll():
        ├─ Info  ──► respond with capacity, versions, mode=1
        └─ Reset ──► if flag & BOOTLOADER: mark run_mode = Service, reset
```


## Step 1: enter service mode

When the user kicks off an update, the host sends `Reset` with the `BOOTLOADER` flag set. The app sees it through `poll()`, writes the "enter bootloader" intent to the BootCtl marker (either a RAM word, BKP register, or BOOT_CTL GPIO depending on the chip), and issues a software reset.

The bootloader starts, reads the marker, and — instead of handing off to the app — goes into service mode and listens on the transport.

```sh
tinyboot reset --bootloader
# device reboots
tinyboot info     # now reports mode=0 (bootloader)
```

## Step 2: flash

`tinyboot flash` drives the four-phase protocol:

1. **Erase** the app region (`META.state` → `Updating`).
2. **Write** the image in 64-byte pages, with `WriteFlags::FLUSH` on the final write.
3. **Verify** — the device CRC16s the image, stores the checksum and size in META, and transitions to `Validating`.
4. **Reset** (if `--reset` was passed) — boot the new app for the first time.

On the first boot under `Validating`, the bootloader consumes one trial and hands off. If the app reaches `confirm()`, META transitions to `Idle` — the update is complete. If the app never confirms (panic, deadlock, bad init), trials get consumed until the bootloader takes back control.

## Trial boot behavior

The trial counter is stored as a byte in META. Each power-on in `Validating` state clears one bit of that byte (a forward-only operation — no erase needed), then boots the app. If the byte reaches zero before `confirm()` lands, the bootloader treats the app as broken and stays in service mode.

This gives you a safety net: a firmware that hangs during init won't brick the device, because the bootloader will reclaim control after the trials run out.

## Probe-flashed apps (development escape hatch)

The "validate app image" step in the lifecycle has two paths. When a CRC is stored in META (the normal post-Verify case), validation runs the CRC. When META is virgin — no update has ever completed through tinyboot — it falls back to a simpler check so an app flashed directly via SWD / JTAG still boots. This lets you iterate on app firmware with a probe without invoking the protocol every time.

All of the following must be true for this path to trigger:

- **Run mode is not Service.** No pending "reboot into bootloader" request from the app.
- **META state is Idle** (`0xFF`). No update is in progress and none has gone through Verify.
- **META checksum is `0xFFFF`.** No CRC has ever been written (freshly-erased META, or a chip that hasn't seen a full tinyboot update yet).
- **App region is non-empty.** The first 32-bit word of the app region is **not** `0xFFFF_FFFF`.

When all four hold, the bootloader hands off to the app. As soon as you run a real tinyboot update cycle (Erase / Write / Verify), META gets a stored checksum and subsequent boots fall back to the normal CRC-validation path.

## What happens if something goes wrong

| Scenario                                         | Outcome                                                                             |
| ------------------------------------------------ | ----------------------------------------------------------------------------------- |
| Power lost during erase or write                 | `META.state = Updating`, app is invalid. Bootloader stays in service mode on restart. |
| Verify returns `CrcMismatch`                     | META stays in `Updating`. Retry or check [troubleshooting](troubleshooting.md).      |
| App panics during init after flash               | Trials run out across reboots; bootloader reclaims control.                          |
| App's `confirm()` never reaches due to bug       | Same as above — trials run out, bootloader wins.                                     |
| Host crashes mid-flash                           | Same as "power lost during erase or write" — safe, just re-flash.                    |

## Making it automatic

Any host logic that can speak the tinyboot CLI can drive updates:

```sh
# from a script
tinyboot reset --bootloader
sleep 0.5
tinyboot flash "$FIRMWARE" --reset
tinyboot info
```

If you're embedding the update flow into a bigger tool, look at the `tinyboot` crate (the CLI) as a starting point — the flash logic there calls into `tinyboot-protocol` directly and can be reused as a library.
