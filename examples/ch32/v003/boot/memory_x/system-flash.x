/* CH32V003 system-flash bootloader memory layout.
 *
 * The bootloader runs from the 1920-byte system flash region.
 * All 16KB of user flash is available for the application.
 * Boot metadata occupies the last page of user flash.
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 2K

    /* Execution mirror of BOOT */
    CODE : ORIGIN = 0x00000000, LENGTH = 1920

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x1FFFF000, LENGTH = 1920
    APP  : ORIGIN = 0x08000000, LENGTH = 16K - 64
    META : ORIGIN = 0x08000000 + 16K - 64, LENGTH = 64
}
