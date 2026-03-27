/* CH32V003 system-flash bootloader memory layout.
 *
 * The bootloader runs from the 1920-byte system flash region (0x1FFFF000).
 * On reset, system flash is mapped at 0x00000000 for code execution (CODE),
 * while programming requires the physical address (FLASH).
 *
 * This frees all 16KB of user flash (0x08000000) for the application.
 * Boot metadata (state, trial counter) is stored in option bytes (0x1FFFF800).
 */
MEMORY
{
    CODE  : ORIGIN = 0x00000000, LENGTH = 1920
    FLASH : ORIGIN = 0x1FFFF000, LENGTH = 1920
    RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}
