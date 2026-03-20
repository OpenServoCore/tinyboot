# tinyboot-protocol

Wire protocol for tinyboot. Defines the frame format used between host and device over UART/RS-485.

## Frame format

A single `Frame` struct is used for both requests (host to device) and responses (device to host), so we keep code size tiny.

```
 0       1       2       3       4       5       6       7       8       9       10      10+len  10+len+2
 +-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+- - - -+-------+-------+
 | SYNC0 | SYNC1 |  CMD  |STATUS |          ADDR (u32 LE)        | LEN_LO  LEN_HI | DATA... | CRC_LO  CRC_HI |
 | 0xAA  | 0x55  |       |       |                               |                 |         |                 |
 +-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+- - - -+-------+-------+
 |<--------------------- header (10 bytes) --------------------->|<- payload ->|<--- CRC --->|
```

Total frame size = 12 bytes overhead + payload. Maximum payload is 64 bytes (`MAX_PAYLOAD`).

| Field  | Size     | Description                                                   |
| ------ | -------- | ------------------------------------------------------------- |
| SYNC   | 2 bytes  | Preamble `0xAA 0x55` for frame synchronization                |
| CMD    | 1 byte   | Command code                                                  |
| STATUS | 1 byte   | `Request (0x00)` for requests, result status for responses    |
| ADDR   | 4 bytes  | Flash address (u32 LE). Echoed in responses                   |
| LEN    | 2 bytes  | Data payload length (u16 LE, 0..64)                           |
| DATA   | 0..64    | Payload bytes                                                 |
| CRC    | 2 bytes  | CRC16-CCITT (LE) over SYNC + CMD + STATUS + ADDR + LEN + DATA |

## Commands

| Code | Name   | Direction      | Description                                               |
| ---- | ------ | -------------- | --------------------------------------------------------- |
| 0x00 | Info   | Host to Device | Query device info (capacity, erase size, versions, mode)  |
| 0x01 | Erase  | Host to Device | Erase `byte_count` bytes at addr (first erase transitions Idle → Updating) |
| 0x02 | Write  | Host to Device | Write data at address                                     |
| 0x03 | Verify | Host to Device | Compute CRC16, store checksum + Validating state in OB    |
| 0x04 | Reset  | Host to Device | Reset the device                                          |

### Info response

Returns 12 bytes via the `InfoData` struct:

| Offset | Size    | Field          | Description                                |
| ------ | ------- | -------------- | ------------------------------------------ |
| 0      | 4 bytes | capacity       | App region capacity in bytes (u32 LE)      |
| 4      | 2 bytes | erase_size     | Erase page size in bytes (u16 LE)          |
| 6      | 2 bytes | boot_version   | Boot version (packed u16 LE, 0xFFFF=none)  |
| 8      | 2 bytes | app_version    | App version (packed u16 LE, 0xFFFF=none)   |
| 10     | 2 bytes | mode           | 0 = bootloader, 1 = app                    |

Versions are packed as `(major << 11) | (minor << 6) | patch` and read from the last 2 bytes of each flash region.

### Erase

Erases `byte_count` bytes starting at `addr`. Both `addr` and `byte_count` must be aligned to the device's erase size.

Request payload (2 bytes via `EraseData`):

| Offset | Size    | Field      | Description                          |
| ------ | ------- | ---------- | ------------------------------------ |
| 0      | 2 bytes | byte_count | Number of bytes to erase (u16 LE)    |

### Verify response

Returns 2 bytes via the `VerifyData` struct:

| Offset | Size    | Description                     |
| ------ | ------- | ------------------------------- |
| 0      | 2 bytes | CRC16 of app region (u16 LE)    |

## Status codes

| Code | Name            | Description                         |
| ---- | --------------- | ----------------------------------- |
| 0x00 | Request         | Frame is a request (not a response) |
| 0x01 | Ok              | Success                             |
| 0x02 | WriteError      | Flash write/erase failed            |
| 0x03 | CrcMismatch     | CRC verification failed             |
| 0x04 | AddrOutOfBounds | Address or length out of range      |
| 0x05 | Unsupported     | Command not valid in current state  |

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

Host  -> Verify
Device -> Ok crc=0x1234

Host  -> Reset
Device -> Ok (then resets)
```
