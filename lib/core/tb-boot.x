/* tinyboot boot-side linker fragment.
 *
 * Expects memory.x to define CODE, BOOT, APP, META, and RAM regions.
 * Add -Ttb-boot.x to linker flags in your bootloader binary. */

/* Symbols derived from memory regions. */
__tb_app_base = ORIGIN(APP);
__tb_app_entry = LENGTH(BOOT);
__tb_meta_base = ORIGIN(META);

/* Boot version at last 2 bytes of boot flash.
 * VMA set explicitly so the CODE mirror matches the BOOT LMA. */
SECTIONS
{
    .tb_version (ORIGIN(CODE) + LENGTH(CODE) - 2) : AT(ORIGIN(BOOT) + LENGTH(BOOT) - 2)
    {
        __tb_version = .;
        KEEP(*(.tb_version));
    } > CODE
}
