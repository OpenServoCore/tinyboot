/* CH32V003 application memory layout for system-flash bootloader.
 *
 * The application occupies all user flash minus the last page (boot metadata).
 * The bootloader lives in the separate system flash region.
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 2K

    /* Execution mirror of APP */
    CODE : ORIGIN = 0x00000000, LENGTH = 16K - 64

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x1FFFF000, LENGTH = 1920
    APP  : ORIGIN = 0x08000000, LENGTH = 16K - 64
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
