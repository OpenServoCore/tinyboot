# Troubleshooting

Common symptoms and their usual causes. If you hit something not covered here, please [open an issue](https://github.com/OpenServoCore/tinyboot/issues).

> [!NOTE]
> This page is a skeleton. Each section lists the likely causes and fixes at a high level; the detailed step-by-step is being re-validated and will land in a follow-up pass.

---

## `tinyboot flash` fails at Verify with `CrcMismatch`

The image reached the device but the CRC over flash didn't match. Common causes:

- `WriteFlags::FLUSH` was not set on the final write of a contiguous region.
- A write payload was not padded to a 4-byte boundary.
- The host skipped an address gap without flushing the previous region first.

If you're using the shipped `tinyboot` CLI, the above are handled for you. If you've written a custom host tool, re-check those three rules against the [protocol reference](https://github.com/OpenServoCore/tinyboot/tree/main/lib/protocol).

---

## Device does not respond to `tinyboot info`

Something in the UART chain isn't right. Work through these in order:

- **Baud rate** — host and device must match. CLI default is 115200.
- **Pins** — the bootloader `UsartMapping` must match how your UART is wired (and, for the app, the pins passed to `Uart::new_blocking`).
- **`rx_pull`** — floating RX lines need `Pull::Up`; externally pulled-up lines should use `Pull::None`.
- **RS-485 / DXL TTL** — a DE/RE pin must be configured via `TxEnConfig`, with `tx_level` matching the transceiver's DE polarity.
- **Half-duplex contention** — on some boards the programmer's TX driver and the MCU's TX driver both reach MCU_RX. Flipping `tx_level` (so the MCU's side is tri-stated while idle) often resolves it. See the [transports guide](transports.md).

---

## Bootloader changes don't take effect after `wlink flash`

After writing to system flash on some chips, a full power cycle is required before the new bootloader runs. A software reset or re-attach is not enough.

- Toggle VCC, or use `wlink set-power disable3v3` followed by `wlink set-power enable3v3`.

---

## CH32V103 won't boot / the debugger can't attach

The option bytes may be corrupted or the debug interface may be held off by the running firmware. Recovery path:

1. Hold BOOT0 and BOOT1 both HIGH during a reset to force SRAM boot.
2. With the chip still held in SRAM boot, run `wlink unprotect` to clear the read / write protection on the option bytes.
3. Release BOOT0 / BOOT1 and power-cycle. `wlink` can then reflash normally.

---

## `wchisp` can't enter boot mode

`wchisp` drives the factory UART ISP, which lives in system flash. A few reasons it might fail:

- **You're in system-flash mode** — tinyboot has overwritten the factory ISP, so `wchisp` has nothing to talk to. Use `wlink` over SWIO instead, or reflash a factory image to system flash from [`vendor/`](https://github.com/OpenServoCore/tinyboot/tree/main/vendor) first.
- **CH32V103** — entry needs BOOT0 HIGH plus BOOT1 LOW (PB2 LOW) at reset.
- **CH32V003 / V00x** — no hardware entry condition; the factory ISP is normally reached via a software stub that jumps to it. In practice, use `wlink` for anything outside of a fresh-chip ISP workflow.

---

## `wlink erase` didn't wipe the bootloader

`wlink erase` only targets the **user flash** region. Bootloaders living in system flash (CH32V003 / V00x / V103 default in this repo) are untouched.

- To fully reset: `wlink erase` to wipe user flash, then re-flash the bootloader to system flash (either your tinyboot build or a factory image from [`vendor/`](https://github.com/OpenServoCore/tinyboot/tree/main/vendor)), then power-cycle.

---

## App crashes immediately on hand-off from the bootloader (user-flash mode)

This is specific to **user-flash mode**, where the app lives behind the bootloader in user flash (e.g. at `0x08000800`) rather than at the start of the mapped execution region. In system-flash mode the app starts at `0x08000000` which is already mapped to `0x0` at reset, so this doesn't apply.

When the app links against `qingke-rt`, the runtime hardcodes the trap vector base (`mtvec`) to `0x0`. Behind-a-bootloader apps need the vector to point to their own `.trap` section instead.

- The example apps work around this via a linker `--wrap` flag applied to the relevant `qingke-rt` symbol. Copy the wiring from [`examples/ch32/v003/app/`](https://github.com/OpenServoCore/tinyboot/tree/main/examples/ch32/v003/app) when starting your own user-flash-mode app crate.

---

## BOOT0 stays HIGH after a soft reset during a user-flash test

On CH32V103 with the BOOT_CTL RC circuit attached, a soft reset can leave BOOT0 latched HIGH long enough that the chip boots into system flash even in user-flash test mode.

- When testing user-flash-mode tinyboot on a V103 board that has the RC network, temporarily disconnect the PB1 → BOOT0 network.

---

## I bricked my V103 and can't recover

See the SRAM-boot recovery procedure above — BOOT0 HIGH + BOOT1 HIGH on reset puts the chip in a state where debuggers can attach even if option bytes are corrupted.

---

Contributions to this page are especially welcome. If you've hit and fixed something that isn't covered, please open a PR.
