/* tinyboot app-side linker fragment.
 *
 * Expects memory.x to define FLASH and RAM regions.
 * Add -Ttb-app.x to linker flags in your application binary. */

/* App version placed immediately after all other flash content.
 * The bootloader reads it at storage[app_size - 2]. */
SECTIONS
{
    .tinyboot_version ALIGN(2) :
    {
        __tinyboot_version = .;
        KEEP(*(.tinyboot_version));
    } > FLASH
} INSERT AFTER .data;
