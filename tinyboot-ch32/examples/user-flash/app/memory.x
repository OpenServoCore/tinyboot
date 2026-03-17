/* CH32V003 application memory layout for user-flash bootloader.
 *
 * The application occupies the upper 12KB of user flash, starting after the
 * bootloader+meta region.
 *
 * Flash map (see boot/memory.x for the full picture):
 *   0x0000_0000 .. 0x0000_0FBF  bootloader  (4KB - 64)
 *   0x0000_0FC0 .. 0x0000_0FFF  boot meta   (64 bytes)
 *   0x0000_1000 .. 0x0000_3FFF  application (12KB)
 *
 * Note: addresses here use the code-execution alias (0x0000_0000 base).
 * The FPEC programming offset (0x0800_0000) is handled by the HAL.
 */
MEMORY
{
    FLASH : ORIGIN = 0x00001000, LENGTH = 12K
    RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}

/* Region aliases required by qingke-rt's linker script. */
REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
