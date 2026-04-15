/* CH32V103 application memory layout for system-flash bootloader.
 *
 * The application occupies all user flash minus the last page (boot metadata).
 * The bootloader lives in the separate system flash region.
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 20K

    /* Execution mirror of APP */
    CODE : ORIGIN = 0x00000000, LENGTH = 64K - 128

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x1FFFF000, LENGTH = 2048
    APP  : ORIGIN = 0x08000000, LENGTH = 64K - 128
    META : ORIGIN = 0x08000000 + 64K - 128, LENGTH = 128
}

/* qingke-rt expects a FLASH region */
REGION_ALIAS("FLASH", CODE);

/* Region aliases required by qingke-rt */
REGION_ALIAS("REGION_TEXT", CODE);
REGION_ALIAS("REGION_RODATA", CODE);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
