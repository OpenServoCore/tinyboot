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

Total frame size = 12 bytes overhead + payload. Maximum payload is 64 bytes (`MAX_PAYLOAD`).

| Field  | Size    | Description                                                           |
| ------ | ------- | --------------------------------------------------------------------- |
| SYNC   | 2 bytes | Preamble `0xAA 0x55` for frame synchronization                        |
| CMD    | 1 byte  | Command code                                                          |
| STATUS | 1 byte  | `Request (0x00)` for requests, result status for responses            |
| ADDR   | 3 bytes | Flash address (u24 LE). Echoed in responses                           |
| FLAGS  | 1 byte  | Per-command flags (see below). Occupies addr byte 3                   |
| LEN    | 2 bytes | Data payload length (u16 LE, 0..64)                                   |
| DATA   | 0..64   | Payload bytes                                                         |
| CRC    | 2 bytes | CRC16-CCITT (LE) over SYNC + CMD + STATUS + ADDR + FLAGS + LEN + DATA |

## Commands

| Code | Name   | Direction      | Description                                                                |
| ---- | ------ | -------------- | -------------------------------------------------------------------------- |
| 0x00 | Info   | Host to Device | Query device info (capacity, erase size, versions, mode)                   |
| 0x01 | Erase  | Host to Device | Erase `byte_count` bytes at addr (first erase transitions Idle → Updating) |
| 0x02 | Write  | Host to Device | Write data at address. `WriteFlags::FLUSH` commits trailing partial page   |
| 0x03 | Verify | Host to Device | Compute CRC16 over `addr` bytes of app, store checksum + state in metadata |
| 0x04 | Reset  | Host to Device | Reset the device. `ResetFlags::BOOTLOADER` enters bootloader (service)     |

## Per-command flags

Byte 3 of the address field carries per-command flags. Each command defines its own bitflags type; unused bits are reserved and must be zero.

### `WriteFlags` (Cmd::Write)

| Bit | Flag    | Description                                                                                                                                                           |
| --- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 7   | `FLUSH` | Commit the buffered page after this write and reset write state. Required on the last write of each contiguous region (before an address jump or at end of transfer). |

### `ResetFlags` (Cmd::Reset)

| Bit | Flag         | Description                                     |
| --- | ------------ | ----------------------------------------------- |
| 0   | `BOOTLOADER` | Enter the bootloader (service mode) after reset |

With no flag set, `Cmd::Reset` boots the app (hand-off mode).

### Info response

Returns 12 bytes via the `InfoData` struct:

| Offset | Size    | Field        | Description                               |
| ------ | ------- | ------------ | ----------------------------------------- |
| 0      | 4 bytes | capacity     | App region capacity in bytes (u32 LE)     |
| 4      | 2 bytes | erase_size   | Erase page size in bytes (u16 LE)         |
| 6      | 2 bytes | boot_version | Boot version (packed u16 LE, 0xFFFF=none) |
| 8      | 2 bytes | app_version  | App version (packed u16 LE, 0xFFFF=none)  |
| 10     | 2 bytes | mode         | 0 = bootloader, 1 = app                   |

Versions are packed as `(major << 11) | (minor << 6) | patch` and read from the last 2 bytes of each binary (boot and app).

### Erase

Erases `byte_count` bytes starting at `addr`. Both `addr` and `byte_count` must be aligned to the device's erase size.

Request payload (2 bytes via `EraseData`):

| Offset | Size    | Field      | Description                       |
| ------ | ------- | ---------- | --------------------------------- |
| 0      | 2 bytes | byte_count | Number of bytes to erase (u16 LE) |

### Flushing buffered writes

Writes are accumulated in a ring buffer on the device and flushed a page at a time. The host sets `WriteFlags::FLUSH` on the Write that ends a contiguous region to commit the trailing partial page. Flush is required:

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

## CRC

CRC16-CCITT with polynomial `0x1021` and initial value `0xFFFF`. Computed over the entire frame body (SYNC through DATA, excluding the CRC field itself). Bit-bang implementation with no lookup table for minimal flash footprint.

## Protocol flow

1. Host sends a request frame with `status = Request (0x00)`
2. Device reads the frame, processes the command
3. Device sends a response frame with `cmd` and `addr` echoed from the request, `status` set to the result

The same `Frame` struct is reused: after `read()`, the device modifies `status`, `len`, and `data`, then calls `send()`. The `cmd` and `addr` fields carry over automatically.

## Data union

The `data` field is a `#[repr(C)]` union with typed variants for structured payloads:

```rust
pub union Data {
    pub raw: [u8; MAX_PAYLOAD],
    pub info: InfoData,
    pub erase: EraseData,
    pub verify: VerifyData,
}
```

Data starts at offset 10 (even-aligned), so `u16` fields in the union variants are naturally aligned.

## Example: flash sequence

```
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
