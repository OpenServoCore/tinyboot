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

/* qingke-rt's link.x doesn't define a .uninit section.
 * defmt-rtt places its buffer in .uninit.* and expects NOLOAD. */
SECTIONS
{
    .uninit (NOLOAD) : ALIGN(4)
    {
        *(.uninit .uninit.*);
    } > RAM
} INSERT AFTER .bss;
