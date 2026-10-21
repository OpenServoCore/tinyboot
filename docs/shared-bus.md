# Shared bus (addressable)

By default a tinyboot device answers every frame it hears. That is fine with one device on the wire. But with several devices on one bus (a chain of servos, a row of sensor boards on RS-485), they would all answer at once.

An **addressable** device stays silent until the host selects it by its unique id. The bootloader and the app both support it, and the CLI drives it for you.

## The short version

```sh
# List every device on the bus by unique id.
tinyboot discover

# Talk to one of them. A unique prefix of the id is enough.
tinyboot info --uid 1032
tinyboot flash new-firmware.elf --uid 1032 --reset
```

With a single device on the bus you don't need `--uid` at all. The CLI selects the only device by itself, and falls back to the plain flow when the device is not addressable.

## Turning it on

In the bootloader, pass `Addressable::new(chip_uid)` to `run()`:

```rust
tinyboot_ch32::boot::run(transport, BootCtl::new(), Addressable::new(chip_uid));
```

`chip_uid` returns the chip's factory unique id (96 bits on CH32, read from the ESIG area). Pass `Unaddressed` instead for a device that has the bus to itself.

In the app, create the handle with `new_addressable_app`:

```rust
let mut app = tinyboot_ch32::app::new_addressable_app(BootCtl::new(), wait_bytes);
```

`wait_bytes(n)` waits about `n` byte-times at the bus baud. The app uses it to space out its `Select` reply (see the slot below). The examples compute it from the USART's baud divisor, so it needs no timer.

Make sure the bootloader and the app agree. If one is addressable and the other is not, the device answers differently depending on which mode it is in.

## The silence rule

An addressable device only speaks in two cases:

- It answers a `Select` whose prefix matches its id.
- It answers any other command while it is selected.

It never answers a frame it can't read (bad CRC, unknown command, payload too big), and it ignores frames that are replies from other devices. Selection lives in RAM, so every reset starts deselected.

This means a device sitting in its bootloader never disturbs the bus. Several devices waiting for firmware at once is fine too. Each one stays quiet until the host picks it.

A device hears every frame on the bus, not just its own. A reply from another device is read and dropped without a CRC check, so the device is listening again right after the last byte. A request is different. Its CRC is checked after the last byte arrives, about 3 us per frame byte on a 48 MHz CH32, and the device cannot hear during that time. The host only sends its next request after the reply or after its listen window, so this never overlaps in normal use. If you write your own host and send a request that draws no reply, leave that much time before the next frame.

## How `Select` works

`Select` carries a seed and a prefix of id bits. Every device whose id starts with the prefix becomes selected and replies with its id. Every other device becomes deselected.

- An empty prefix selects every device.
- A full id selects exactly one device.
- A prefix longer than any id selects none, which is how the host deselects everyone.

The [protocol reference](https://github.com/OpenServoCore/tinyboot/tree/main/lib/protocol#select) has the exact byte layout.

## Finding every device

When two devices reply at the same time, their bytes overlap on the wire and the host reads garble. The host uses that as a signal. `tinyboot discover` walks the id tree one bit at a time:

1. Select a prefix (it starts empty).
2. Silence means no device under this prefix, so this branch is done.
3. Garble means two or more devices, so try the prefix plus `0`, then plus `1`.
4. A clean reply is one device. The host double checks it by probing the next bit both ways. The device has to answer alone under one and nothing under the other.

The double check is there because devices running the same code can answer in exact unison. On some wires that can read back as one clean frame. Each device waits a **slot** before replying, 0 to 15 byte-times picked from a hash of its id and the seed. The host sends a new seed with every probe, so two devices that collided once get a fresh draw next time.

Each probe listens about 50 ms for replies. Devices whose ids part early are found in a few probes each. Two ids that share a long prefix take longer, since the walk goes down that prefix one bit at a time. The worst case, two 96-bit ids that differ only in their last bit, takes about 150 probes, so several seconds.

## Custom transports

The slot wait goes through `Transport::wait_bytes`. The CH32 USART implements it by spinning on the baud divisor. If you write your own transport for a shared bus, implement it too. The default returns right away, which still works, it just gives up the unison protection.

## What it costs

On CH32V003 in system flash, addressable adds about 130 bytes. Every combination of wire setup, DE pin, compatibility key and addressing still fits the 1920-byte system flash. CI builds the heaviest one (`examples/ch32/v003/boot/src/bin/heaviest.rs`) to keep it that way.

## Caveats

- Don't mix addressable and unaddressed devices on one bus. An unaddressed device answers everything, including `Select`, so it garbles every probe.
- Two devices with the same id can't be told apart. The walk stops with an error. Chip unique ids are unique, so this only happens with a custom id source.
- The CLI's `--uid` takes whole bytes (2 hex digits each), in the order `tinyboot discover` prints them.
