/* CH32V003 application memory layout for system-flash bootloader.
 *
 * The application occupies the full 16KB user flash. The bootloader lives
 * in the separate system flash region, so no space is reserved here.
 *
 * Addresses use the code-execution alias (0x0000_0000 base).
 * The FPEC programming offset (0x0800_0000) is handled by the HAL.
 */
MEMORY
{
    FLASH : ORIGIN = 0x00000000, LENGTH = 16K
    RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}

/* Region aliases required by qingke-rt's linker script. */
REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
