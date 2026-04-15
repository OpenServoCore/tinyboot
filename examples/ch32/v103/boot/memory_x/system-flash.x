/* CH32V103 system-flash bootloader memory layout.
 *
 * The bootloader runs from the first 2048-byte system flash region.
 * All 64KB of user flash is available for the application.
 * Boot metadata occupies the last page of user flash.
 *
 * System flash is split by option bytes:
 *   0x1FFFF000 .. 0x1FFFF7FF  region 1    2048 bytes (used)
 *   0x1FFFF800 .. 0x1FFFF8FF  option bytes (gap)
 *   0x1FFFF900 .. 0x1FFFFFFD  region 2    1792 bytes (unused for now)
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 20K

    /* Execution mirror of BOOT */
    CODE : ORIGIN = 0x00000000, LENGTH = 2048

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x1FFFF000, LENGTH = 2048
    APP  : ORIGIN = 0x08000000, LENGTH = 64K - 128
    META : ORIGIN = 0x08000000 + 64K - 128, LENGTH = 128
}
