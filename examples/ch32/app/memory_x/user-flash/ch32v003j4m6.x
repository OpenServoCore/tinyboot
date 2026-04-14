/* CH32V003 application memory layout for user-flash bootloader.
 *
 * The application occupies user flash between the bootloader and
 * boot metadata regions.
 *
 * Flash map (see boot/memory.x for the full picture):
 *   0x0000_0000 .. 0x0000_1FFF  bootloader   8KB
 *   0x0000_2000 .. 0x0000_3FBF  application  8KB - 64B
 *   0x0000_3FC0 .. 0x0000_3FFF  boot meta    64B
 *
 * Note: addresses here use the code-execution alias (0x0000_0000 base).
 * The FPEC programming offset (0x0800_0000) is handled by the HAL.
 */
MEMORY
{
    FLASH : ORIGIN = 0x00000000 + 8K, LENGTH = 8K - 64
    META  : ORIGIN = 0x00000000 + 16K - 64, LENGTH = 64
    RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}

/* Region aliases required by qingke-rt's linker script. */
REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);

__tinyboot_meta_start = ORIGIN(META) + 0x08000000;
