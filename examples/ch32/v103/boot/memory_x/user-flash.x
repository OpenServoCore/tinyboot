/* CH32V103 user-flash bootloader memory layout.
 *
 * The bootloader occupies the first 8KB of the 64KB user flash.
 * Boot metadata occupies the last page of user flash.
 *
 * Flash map:
 *   0x0800_0000 .. 0x0800_1FFF  bootloader   8KB
 *   0x0800_2000 .. 0x0800_FF7F  application  56KB - 128B
 *   0x0800_FF80 .. 0x0800_FFFF  boot meta    128B
 */
MEMORY
{
    RAM  : ORIGIN = 0x20000000, LENGTH = 20K

    /* Execution mirror of BOOT */
    CODE : ORIGIN = 0x00000000, LENGTH = 8K

    /* Physical flash addresses */
    BOOT : ORIGIN = 0x08000000, LENGTH = 8K
    APP  : ORIGIN = 0x08000000 + 8K, LENGTH = 56K - 128
    META : ORIGIN = 0x08000000 + 64K - 128, LENGTH = 128
}
