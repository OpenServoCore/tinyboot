# Vendor Bootloader Binaries

Factory system flash images for restoring the vendor bootloader. Useful if you've
overwritten system flash and need to recover.

## Files

| File                          | Chip         | Address      | Size       |
| ----------------------------- | ------------ | ------------ | ---------- |
| `ch32v003-system-flash.bin`   | CH32V003F4P6 | `0x1FFFF000` | 1920 bytes |
| `ch32v103-system-flash-1.bin` | CH32V103     | `0x1FFFF000` | 2048 bytes |
| `ch32v103-system-flash-2.bin` | CH32V103     | `0x1FFFF900` | 1792 bytes |

## Restoring

```sh
wlink flash vendor/ch32v003-system-flash.bin --address 0x1FFFF000

# Ch32V103 has split system flash regions, so we need to flash each region separately.
wlink flash vendor/ch32v103-system-flash-1.bin --address 0x1FFFF000
wlink flash vendor/ch32v103-system-flash-2.bin --address 0x1FFFF900
```
