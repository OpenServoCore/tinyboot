# For ch32v003 series
# They all have 2k RAM and 16k flash.
MEMORY
{
  RAM   : ORIGIN = 0x20000000, LENGTH = 2K
  BOOT  : ORIGIN = 0x00000000, LENGTH = 4K - 64
  META  : ORIGIN = 0x00000FC0, LENGTH = 64
  APP   : ORIGIN = 0x00001000, LENGTH = 16k - 4k
}

REGION_ALIAS("FLASH",         BOOT);
REGION_ALIAS("REGION_TEXT",   BOOT);
REGION_ALIAS("REGION_RODATA", BOOT);
REGION_ALIAS("REGION_DATA",   RAM);
REGION_ALIAS("REGION_BSS",    RAM);
REGION_ALIAS("REGION_HEAP",   RAM);
REGION_ALIAS("REGION_STACK",  RAM);

# Used by bootloader to:
# - know where the app is located in flash so it can jump to it after booting
# - validate flash file size before writing to flash
# - read boot metadata to determine boot or app mode
__APP_ADDR  = ORIGIN(APP);
__APP_SIZE = LENGTH(APP);
__META_ADDR = ORIGIN(META);
