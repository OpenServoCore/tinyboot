/* CH32V003 application memory layout for user-flash bootloader.
 *
 * The application occupies user flash between the bootloader and
 * boot metadata regions.
 *
 * Flash map:
 *   0x0000_0000 .. 0x0000_07FF  bootloader   2KB
 *   0x0000_0800 .. 0x0000_3FBF  application  14KB - 64B
 *   0x0000_3FC0 .. 0x0000_3FFF  boot meta    64B
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 2K

    /* Execution mirror of APP */
    CODE : ORIGIN = 0x00000000 + 2K, LENGTH = 14K - 64

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x08000000, LENGTH = 2K
    APP  : ORIGIN = 0x08000000 + 2K, LENGTH = 14K - 64
    META : ORIGIN = 0x08000000 + 16K - 64, LENGTH = 64
}

/* qingke-rt expects a FLASH region */
REGION_ALIAS("FLASH", CODE);

/* Region aliases required by qingke-rt (from ch32-metapac's memory.x, with FLASH replaced by CODE) */
REGION_ALIAS("REGION_TEXT", CODE);
REGION_ALIAS("REGION_RODATA", CODE);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
