# tinyboot-protocol

Wire protocol for tinyboot. Defines the frame format used between host and device over UART/RS-485.

## Frame format

A single `Frame` struct is used for both requests (host to device) and responses (device to host), so we keep code size tiny. The data buffer size is determined by the transport via a const generic `D`.

```
 0       1       2       3       4       5       6       7       8       9       10      10+len  10+len+2
 +-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+- - - -+-------+-------+
 | SYNC0 | SYNC1 |  CMD  |STATUS |          ADDR (u32 LE)        | LEN_LO  LEN_HI | DATA... | CRC_LO  CRC_HI |
 | 0xAA  | 0x55  |       |       |                               |                 |         |                 |
 +-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+-------+- - - -+-------+-------+
 |<--------------------- header (10 bytes) --------------------->|<- payload ->|<--- CRC --->|
```

Total frame size = 12 bytes overhead + payload. For example, a UART transport with 64-byte frames has 52 bytes of payload per frame.

| Field  | Size     | Description                                                   |
| ------ | -------- | ------------------------------------------------------------- |
| SYNC   | 2 bytes  | Preamble `0xAA 0x55` for frame synchronization                |
| CMD    | 1 byte   | Command code                                                  |
| STATUS | 1 byte   | `Request (0x00)` for requests, result status for responses    |
| ADDR   | 4 bytes  | Flash address (u32 LE). Echoed in responses                   |
| LEN    | 2 bytes  | Data payload length (u16 LE, 0..D)                            |
| DATA   | 0..D     | Payload bytes                                                 |
| CRC    | 2 bytes  | CRC16-CCITT (LE) over SYNC + CMD + STATUS + ADDR + LEN + DATA |

## Commands

| Code | Name   | Direction      | Description                                               |
| ---- | ------ | -------------- | --------------------------------------------------------- |
| 0x01 | Info   | Host to Device | Query device geometry (capacity, payload size, erase size) |
| 0x02 | Erase  | Host to Device | Erase one page at addr                                    |
| 0x03 | Write  | Host to Device | Write data at address                                     |
| 0x04 | Verify | Host to Device | Compute CRC16 over app region                             |
| 0x05 | Reset  | Host to Device | Advance boot state and reset                              |

### Info response

Returns 8 bytes via the `InfoData` union variant:

| Offset | Size    | Description                                |
| ------ | ------- | ------------------------------------------ |
| 0      | 4 bytes | App region capacity in bytes (u32 LE)      |
| 4      | 2 bytes | Max payload size per write frame (u16 LE)  |
| 6      | 2 bytes | Erase page size in bytes (u16 LE)          |

### Erase

Erases one page at `addr`. The address must be aligned to the device's erase size. The host should loop over all pages to erase the full region.

### Verify response

Returns 2 bytes via the `VerifyData` union variant:

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

## CRC

CRC16-CCITT with polynomial `0x1021` and initial value `0xFFFF`. Computed over the entire frame body (SYNC through DATA, excluding the CRC field itself). Bit-bang implementation with no lookup table for minimal flash footprint.

## Protocol flow

1. Host sends a request frame with `status = Request (0x00)`
2. Device reads the frame, processes the command
3. Device sends a response frame with `cmd` and `addr` echoed from the request, `status` set to the result

The same `Frame` struct is reused: after `read()`, the device modifies `status`, `len`, and `data`, then calls `send()`. The `cmd` and `addr` fields carry over automatically.

## Data union

The `data` field is a `#[repr(C)]` union with typed variants for structured responses:

```rust
pub union Data<const D: usize> {
    pub raw: [u8; D],
    pub info: InfoData,
    pub verify: VerifyData,
}
```

Data starts at offset 10 (even-aligned), so `u16` fields in the union variants are naturally aligned.

## Example: flash sequence

```
Host  -> Info request
Device -> Info response (capacity=16384, payload_size=52, erase_size=64)

Host  -> Erase addr=0x0000
Device -> Ok
Host  -> Erase addr=0x0040
Device -> Ok
...

Host  -> Write addr=0x0000 data=[52 bytes]
Device -> Ok
Host  -> Write addr=0x0034 data=[52 bytes]
Device -> Ok
...

Host  -> Verify
Device -> Ok crc=0x1234

Host  -> Reset
Device -> Ok (then resets)
```
