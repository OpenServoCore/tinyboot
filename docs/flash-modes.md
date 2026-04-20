# Flash modes: system-flash vs user-flash

CH32 parts have two regions of on-chip flash:

- **System flash** — a small region at `0x1FFF_xxxx`, normally containing the factory ISP bootloader. On CH32, this region is writable via `wlink` and can host tinyboot.
- **User flash** — the main application flash starting at `0x0800_0000`, mapped to `0x0000_0000` at execution.

tinyboot supports running from either region on every supported chip. This page explains the tradeoffs.

## Quick recommendation

- **Default to `system-flash`** when your chip can switch boot source in software. The entire user flash stays available for your application.
- **Use `user-flash`** if you'd rather avoid the BOOT_CTL circuit some chips need for system-flash, or keep the factory ISP in place for easier recovery.

Picking a mode is controlled by a Cargo feature on your **boot** crate — the app crate doesn't need to know.

## Mode capacities

| Chip family | System flash size | User flash size | System-flash mode | User-flash mode |
| ----------- | ----------------- | --------------- | ----------------- | --------------- |
| CH32V003    | 1920 B            | 16 KB           | ✅ Supported      | ✅ Supported    |
| CH32V00x    | 3 KB + 256 B      | 16–64 KB        | ✅ Supported      | ✅ Supported    |
| CH32V103    | 2 KB (+ 1.75 KB split) | 32–64 KB   | ✅ Supported      | ✅ Supported    |

> [!NOTE]
> Chips that can't switch boot source via a software MODE register need an external BOOT_CTL circuit for system-flash mode — see [boot-ctl.md](boot-ctl.md). Among supported chips, this currently applies to **CH32V103**. V003 / V00x switch boot source in software and need no extra hardware.

> [!NOTE]
> **CH32V103 split system flash**: option bytes sit in the middle of system flash, splitting it into a 2 KB primary region at `0x1FFFF000` and a 1.75 KB secondary region at `0x1FFFF900`. V103 system-flash `memory.x` declares both as `BOOT` / `BOOT2` (with matching `CODE` / `CODE2` execution mirrors) so the linker can spill overflow into the secondary region if needed.

## What's in each region

Regardless of mode, every tinyboot layout reserves four regions, named in [`memory.x`](porting.md#linker-region-contract):

| Region | Role                                  | Where it lives                                     |
| ------ | ------------------------------------- | -------------------------------------------------- |
| `BOOT` | Bootloader code                       | System flash (system-flash mode) or top/bottom of user flash (user-flash mode) |
| `APP`  | Application code                      | User flash                                         |
| `META` | Boot metadata (state, trials, CRC)    | Last page of user flash                            |
| `CODE` | Execution mirror (VMA) of `BOOT`      | Usually `0x0000_0000` for boot, `0x0000_0000` + offset for app |

The `META` page is always in user flash, even in system-flash mode — it needs to be on a page boundary that matches the chip's erase granularity, and user flash is always present.

## Choosing a mode

**Choose `system-flash` if:**

- You want all of user flash available for your app.
- You're OK with the small extra wiring on chips that need a BOOT_CTL circuit (V103).
- You don't mind that recovering from a bad bootloader requires `wlink` + a power cycle.

**Choose `user-flash` if:**

- You want the factory ISP left intact in system flash for easy recovery via `wchisp`.
- You want a uniform layout across a fleet where some chips don't have BOOT_CTL wired.
- You want the bootloader recoverable the same way as the app (any probe, any SWD/JTAG).

## Turning on a mode

The boot crate picks the mode via a Cargo feature:

```toml
[dependencies]
tinyboot-ch32 = { version = "0.4", features = ["ch32v003f4p6", "system-flash"] }
```

Drop `system-flash` to run the bootloader from user flash. The example boot crates expose `system-flash` and `user-flash` as mutually-exclusive features and pick the matching linker script at build time — copy that pattern if you want a single crate that builds both.

See [`examples/ch32/v003/boot/Cargo.toml`](https://github.com/OpenServoCore/tinyboot/blob/main/examples/ch32/v003/boot/Cargo.toml) and [`examples/ch32/v003/boot/memory_x/`](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v003/boot/memory_x) for the full wiring.

## Reverting to the factory bootloader

**In user-flash mode** — the factory ISP in system flash was never touched. Enter it the normal way for your chip (e.g. on V103, hold BOOT0 HIGH and BOOT1 LOW at reset, then use `wchisp` over UART).

**In system-flash mode** — tinyboot has overwritten the factory ISP. Factory images for the supported chips live in [`vendor/`](https://github.com/OpenServoCore/tinyboot/tree/main/vendor) in this repo; reflash them to system flash to restore:

```sh
wlink flash vendor/ch32v003-system-flash.bin   --address 0x1FFFF000
wlink flash vendor/ch32v006-system-flash.bin   --address 0x1FFF0000
# CH32V103 has split system flash — flash each region separately:
wlink flash vendor/ch32v103-system-flash-1.bin --address 0x1FFFF000
wlink flash vendor/ch32v103-system-flash-2.bin --address 0x1FFFF900
wlink set-power disable3v3 && wlink set-power enable3v3
```

See [`vendor/README.md`](https://github.com/OpenServoCore/tinyboot/blob/main/vendor/README.md) for the file / address table.

In practice `wlink` over SWIO is the universal recovery tool — as long as the debug interface is reachable, you can reflash anything in either region regardless of mode.
