/* CH32V003 system-flash bootloader memory layout.
 *
 * The bootloader runs from the 1920-byte system flash region (0x1FFFF000).
 * On reset, system flash is mapped at 0x00000000 for code execution (CODE),
 * while programming requires the physical address (FLASH).
 *
 * This frees all 16KB of user flash (0x08000000) for the application.
 * Boot metadata (state, trial counter) is stored in the last 64 bytes
 * of system flash.
 */
MEMORY
{
    CODE  : ORIGIN = 0x00000000, LENGTH = 1920 - 64   /* execution alias   */
    FLASH : ORIGIN = 0x1FFFF000, LENGTH = 1920 - 64   /* physical address  */
    META  : ORIGIN = 0x1FFFFCC0, LENGTH = 64           /* boot metadata     */
    RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}
