# tinyboot-protocol

Part of the [tinyboot](https://github.com/OpenServoCore/tinyboot) project — start with the [top-level README](https://github.com/OpenServoCore/tinyboot#quick-start-ch32v003) and the [handbook](https://openservocore.github.io/tinyboot/).

Wire protocol for tinyboot. Defines the frame format used between host and device over UART / RS-485. This page is the authoritative wire-format reference; for end-user guides see the [handbook](https://openservocore.github.io/tinyboot/).

## Frame format

A single `Frame` struct is used for both requests (host to device) and responses (device to host), so we keep code size tiny.

```
 0       1       2       3       4       5       6       7       8       9       10      10+len  10+len+2
 +-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+- - - -+-------+-------+
 | SYNC0 | SYNC1 |  CMD  |STATUS |     ADDR (u24 LE)     | FLAGS | LEN_LO  LEN_HI | DATA... | CRC_LO  CRC_HI |
 | 0xAA  | 0x55  |       |       |                       |       |                 |         |                 |
 +-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+- - - -+-------+-------+
 |<--------------------- header (10 bytes) --------------------->|<- payload ->|<--- CRC --->|
```

Total frame size = 12 bytes overhead + payload. The payload capacity is a build choice, `Frame<N>`, 52 bytes by default (`DEFAULT_PAYLOAD`, so a whole frame fits one 64-byte USB full-speed packet); a device reports its own as Info's `max_payload`, and a host sends at most that.

| Field  | Size    | Description                                                           |
| ------ | ------- | --------------------------------------------------------------------- |
| SYNC   | 2 bytes | Preamble `0xAA 0x55` for frame synchronization                        |
| CMD    | 1 byte  | Command code                                                          |
| STATUS | 1 byte  | `Request (0x00)` for requests, result status for responses            |
| ADDR   | 3 bytes | Flash address (u24 LE). Echoed in responses                           |
| FLAGS  | 1 byte  | Per-command flags (see below). Occupies addr byte 3                   |
| LEN    | 2 bytes | Data payload length (u16 LE, 0..N, N the receiver's payload capacity) |
| DATA   | 0..N    | Payload bytes                                                         |
| CRC    | 2 bytes | CRC16-CCITT (LE) over SYNC + CMD + STATUS + ADDR + FLAGS + LEN + DATA |

## Commands

| Code | Name   | Direction      | Description                                                                                                                                                                              |
| ---- | ------ | -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0x00 | Info   | Host to Device | Query device info (capacity, erase size, versions, mode, baud, compatibility key, payload capacity)                                                                                      |
| 0x01 | Erase  | Host to Device | Erase `byte_count` bytes at addr (first erase transitions Idle → Updating)                                                                                                               |
| 0x02 | Write  | Host to Device | Write data at address. `WriteFlags::FLUSH` commits trailing partial page                                                                                                                 |
| 0x03 | Verify | Host to Device | Compute CRC16 over `addr` bytes of app, store checksum + state in metadata. Refused with `Incompatible` when the bootloader has a compatibility key and the image's tail carries another |
| 0x04 | Reset  | Host to Device | Reset the device. `ResetFlags::BOOTLOADER` enters bootloader (service), `ResetFlags::BAUD` stores a baud first                                                                           |
| 0x05 | Select | Host to Device | Select addressable devices by unique id prefix (see [Select](#select)). A device without addressing answers `Unsupported`                                                                |

## Per-command flags

Byte 3 of the address field carries per-command flags. Each command defines its own bitflags type; unused bits are reserved and must be zero.

### `WriteFlags` (Cmd::Write)

| Bit | Flag    | Description                                                                                                                                                           |
| --- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 7   | `FLUSH` | Commit the buffered page after this write and reset write state. Required on the last write of each contiguous region (before an address jump or at end of transfer). |

### `ResetFlags` (Cmd::Reset)

| Bit | Flag         | Description                                                             |
| --- | ------------ | ----------------------------------------------------------------------- |
| 0   | `BOOTLOADER` | Enter the bootloader (service mode) after reset                         |
| 1   | `BAUD`       | Store addr byte 0 as the bootloader's `BaudRate` index before resetting |

With no flag set, `Cmd::Reset` boots the app (hand-off mode). The bootloader refuses that with `Unsupported` while an update is in progress (META `Updating`), so a half-written image never runs; `BOOTLOADER` is always accepted.

### `BaudRate`

The baud rate travels and is stored as its `u8` index: 0 = 9600, 1 = 19200, 2 = 38400, 3 = 57600, 4 = 115200, 5 = 230400, 6 = 460800, 7 = 500000, 8 = 921600, 9 = 1M, 10 = 1.5M, 11 = 2M, 12 = 2.5M, 13 = 3M. Indices are append-only. The bootloader runs at the stored rate from its next start; an index it does not know, or a rate its clock cannot reach, selects its build default, and so does `0xFF` (erased metadata), which is how `BAUD` with `addr = 0xFF` restores the default.

### Info response

Returns 18 bytes via the `InfoData` struct, in either mode:

| Offset | Size    | Field        | Description                                                    |
| ------ | ------- | ------------ | -------------------------------------------------------------- |
| 0      | 4 bytes | capacity     | App region capacity in bytes (u32 LE)                          |
| 4      | 2 bytes | erase_size   | Erase page size in bytes (u16 LE)                              |
| 6      | 2 bytes | boot_version | Boot version (packed u16 LE, 0xFFFF=none)                      |
| 8      | 2 bytes | app_version  | App version (packed u16 LE, 0xFFFF=none)                       |
| 10     | 1 byte  | mode         | 0 = bootloader, 1 = app                                        |
| 11     | 1 byte  | boot_baud    | Stored bootloader `BaudRate` index, 0xFF = build default       |
| 12     | 4 bytes | compat       | The bootloader's compatibility key (u32 LE), 0xFFFFFFFF = none |
| 16     | 2 bytes | max_payload  | Largest payload the responder accepts in a request (u16 LE)    |

Versions are packed as `(major << 11) | (minor << 6) | patch` and read from the last 2 bytes of each binary (boot and app).

### Image tail

Each boot and app image ends with 6 bytes: the compatibility key (u32 LE), then the version (u16 LE). An app image's tail starts word-aligned, so its size is 2 past a multiple of 4. The key is opaque and compared only for equality; `0xFFFF_FFFF` means none. `compat_key(name)` derives one from a name (32-bit FNV-1a, never `0xFFFF_FFFF`).

### Select

Several addressable devices can share one bus. Each stays silent until the host selects it by its unique id.

Request payload: `[seed, prefix_len, prefix...]`. `prefix_len` counts bits, and `prefix` holds `ceil(prefix_len / 8)` bytes. Bit `k` of an id is `id[k / 8] >> (k % 8) & 1`, with the id stored least significant byte first, so a prefix of whole bytes is the id's leading bytes.

- A device whose id starts with the prefix is selected. It waits `slot` byte-times, then replies `Ok` with its id as data (12 bytes on CH32). `slot = (crc16(0xFFFF, id) >> (seed % 16)) & 15`.
- Every other device is deselected and does not reply.
- An empty prefix selects every device, a full id exactly one, and a `prefix_len` longer than any id none (deselect all).

An addressable device answers `Select` when it matches, and any other command only while selected. It never answers a frame it can't read (bad CRC, unknown command, payload overflow) or a frame whose status is not `Request` (another device's reply). Selection lives in RAM, so a reset deselects it.

Replies from two or more devices at once overlap and read as garble. The slot keeps devices running the same code from answering in exact unison, which on some wires reads back as one clean frame. A fresh `seed` per probe gives a fresh slot. A host finds every device by walking the id tree. Silence ends a branch, garble descends one bit, and a clean reply is a candidate, confirmed when its device answers alone under the next bit of its id and nothing answers under the other.

### Erase

Erases `byte_count` bytes starting at `addr`. Both `addr` and `byte_count` must be aligned to the device's erase size.

Request payload (2 bytes via `EraseData`):

| Offset | Size    | Field      | Description                       |
| ------ | ------- | ---------- | --------------------------------- |
| 0      | 2 bytes | byte_count | Number of bytes to erase (u16 LE) |

### Flushing buffered writes

Writes are accumulated in a one-page buffer on the device and written a page at a time. The host sets `WriteFlags::FLUSH` on the Write that ends a contiguous region to commit the trailing partial page. Flush is required:

- **On the final Write** — otherwise the last partial page may not be written to flash, causing Verify to fail.
- **Before skipping an address range** — if the host advances the write address (e.g. skipping a gap between segments), it must flush the previous region first.

### Write alignment

Write payloads must be padded to a 4-byte boundary. The device writes to flash 4 bytes at a time, so unaligned payloads will fail.

### Verify

The `addr` field carries the application size in bytes. The device computes CRC16 over the first `addr` bytes of flash (the actual firmware, not the full region), stores the checksum and app size in boot metadata, and transitions to Validating state.

If Verify returns `CrcMismatch`, check that all Write payloads were padded to 4 bytes and that `WriteFlags::FLUSH` was set on the last Write (and on the last Write of each contiguous region before any address skip).

Response returns 2 bytes via the `VerifyData` struct:

| Offset | Size    | Description                          |
| ------ | ------- | ------------------------------------ |
| 0      | 2 bytes | CRC16 of app firmware bytes (u16 LE) |

## Status codes

| Code | Name            | Description                         |
| ---- | --------------- | ----------------------------------- |
| 0x00 | Request         | Frame is a request (not a response) |
| 0x01 | Ok              | Success                             |
| 0x02 | WriteError      | Flash write/erase failed            |
| 0x03 | CrcMismatch     | CRC verification failed             |
| 0x04 | AddrOutOfBounds | Address or length out of range      |
| 0x05 | Unsupported     | Command not valid in current state  |
| 0x06 | PayloadOverflow | Frame payload exceeds maximum size  |
| 0x07 | Incompatible    | Image's compatibility key differs   |

## CRC

CRC16-CCITT with polynomial `0x1021` and initial value `0xFFFF`. Computed over the entire frame body (SYNC through DATA, excluding the CRC field itself). Bit-bang implementation with no lookup table for minimal flash footprint.

## Protocol flow

1. Host sends a request frame with `status = Request (0x00)`
2. Device reads the frame, processes the command
3. Device sends a response frame with `cmd` and `addr` echoed from the request, `status` set to the result

The same `Frame` struct is reused: after `read()`, the device modifies `status`, `len`, and `data`, then calls `send()`. The `cmd` and `addr` fields carry over automatically.

An addressable device replies only as [Select](#select) describes, so on a shared bus the host selects a device before anything else.

## Data union

The `data` field is a `#[repr(C)]` union with typed variants for structured payloads:

```rust
pub union Data<const N: usize = DEFAULT_PAYLOAD> {
    pub raw: [u8; N],
    pub info: InfoData,
    pub erase: EraseData,
    pub verify: VerifyData,
}
```

Data starts at offset 10 (even-aligned), so `u16` fields in the union variants are naturally aligned.

## Example: flash sequence

```
Host  -> Select seed=1 prefix_len=0 (addressable devices only)
Device -> Ok uid=[12 bytes]

Host  -> Info request
Device -> Info response (capacity=16384, erase_size=64, mode=0)

Host  -> Erase addr=0x0000 byte_count=16384
Device -> Ok
...

Host  -> Write addr=0x0000 data=[64 bytes]
Device -> Ok
Host  -> Write addr=0x0040 data=[64 bytes]
Device -> Ok
...
Host  -> Write addr=0x13C0 data=[54 bytes] flags=WriteFlags::FLUSH
Device -> Ok

Host  -> Verify addr=5110 (app_size)
Device -> Ok crc=0x1234

Host  -> Reset
Device -> Ok (then resets)
```
