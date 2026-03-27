/* tinyboot boot-side linker fragment.
 *
 * Expects memory.x to define FLASH, CODE, and RAM regions.
 * Add -Ttb-boot.x to linker flags in your bootloader binary. */

/* Boot version at last 2 bytes of boot flash.
 * VMA set explicitly so the CODE mirror matches the FLASH LMA. */
SECTIONS
{
    .tinyboot_version (ORIGIN(CODE) + LENGTH(CODE) - 2) : AT(ORIGIN(FLASH) + LENGTH(FLASH) - 2)
    {
        __tinyboot_version = .;
        KEEP(*(.tinyboot_version));
    } > CODE
}
