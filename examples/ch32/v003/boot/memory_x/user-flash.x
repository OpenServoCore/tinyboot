/* CH32V003 user-flash bootloader memory layout.
 *
 * The bootloader occupies the first 2KB of the 16KB user flash.
 * Boot metadata occupies the last page of user flash.
 *
 * Flash map:
 *   0x0800_0000 .. 0x0800_07FF  bootloader   2KB
 *   0x0800_0800 .. 0x0800_3FBF  application  14KB - 64B
 *   0x0800_3FC0 .. 0x0800_3FFF  boot meta    64B
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 2K

    /* Execution mirror of BOOT */
    CODE : ORIGIN = 0x00000000, LENGTH = 2K

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x08000000, LENGTH = 2K
    APP  : ORIGIN = 0x08000000 + 2K, LENGTH = 14K - 64
    META : ORIGIN = 0x08000000 + 16K - 64, LENGTH = 64
}
