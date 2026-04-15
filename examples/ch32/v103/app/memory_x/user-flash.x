/* CH32V103 application memory layout for user-flash bootloader.
 *
 * The application occupies user flash between the bootloader and
 * boot metadata regions.
 *
 * Flash map:
 *   0x0000_0000 .. 0x0000_1FFF  bootloader   8KB
 *   0x0000_2000 .. 0x0000_FF7F  application  56KB - 128B
 *   0x0000_FF80 .. 0x0000_FFFF  boot meta    128B
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 20K

    /* Execution mirror of APP */
    CODE : ORIGIN = 0x00000000 + 8K, LENGTH = 56K - 128

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x08000000, LENGTH = 8K
    APP  : ORIGIN = 0x08000000 + 8K, LENGTH = 56K - 128
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
