/* CH32V103 system-flash bootloader memory layout.
 *
 * System flash is split into two regions separated by option bytes:
 *
 *   0x1FFFF000 .. 0x1FFFF7FF  region 1    2048 bytes
 *   0x1FFFF800 .. 0x1FFFF8FF  option bytes 256 bytes
 *   0x1FFFF900 .. 0x1FFFFFFD  region 2    1792 bytes (- 2 for version tag)
 *                                   total: 3840 bytes usable
 *
 * On reset, system flash is mapped at 0x00000000 for code execution (CODE/CODE2),
 * while programming requires the physical address (FLASH/FLASH2).
 *
 * This frees all 32KB of user flash (0x08000000) for the application.
 * Boot metadata (state, trial counter) is stored in option bytes (0x1FFFF800).
 */
MEMORY
{
    CODE   : ORIGIN = 0x00000000, LENGTH = 2048
    FLASH  : ORIGIN = 0x1FFFF000, LENGTH = 2048
    CODE2  : ORIGIN = 0x00000900, LENGTH = 1792
    FLASH2 : ORIGIN = 0x1FFFF900, LENGTH = 1792
    RAM    : ORIGIN = 0x20000000, LENGTH = 10K
}
