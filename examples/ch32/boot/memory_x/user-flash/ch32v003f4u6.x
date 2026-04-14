/* CH32V003 user-flash bootloader memory layout.
 *
 * The bootloader occupies the first 8KB of the 16KB user flash.
 * Boot metadata is stored in the last page of user flash.
 *
 * CODE is the execution address (flash mirrored at 0x00000000).
 * FLASH is the programming address (FPEC requires 0x08000000-based addresses).
 *
 * Flash map:
 *   0x0800_0000 .. 0x0800_1FFF  bootloader   8KB
 *   0x0800_2000 .. 0x0800_3FBF  application  8KB - 64B
 *   0x0800_3FC0 .. 0x0800_3FFF  boot meta    64B
 */
MEMORY
{
    CODE  : ORIGIN = 0x00000000, LENGTH = 8K  /* execution alias   */
    FLASH : ORIGIN = 0x08000000, LENGTH = 8K  /* physical address  */
    RAM   : ORIGIN = 0x20000000, LENGTH = 2K
}

__tinyboot_meta_start = 0x08000000 + 16K - 64;
