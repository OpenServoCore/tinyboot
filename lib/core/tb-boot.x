/* tinyboot boot-side linker fragment.
 *
 * Expects memory.x to define CODE, BOOT, APP, META, and RAM regions.
 * Add -Ttb-boot.x to linker flags in your bootloader binary. */

/* Symbols derived from memory regions. */
__tb_app_base = ORIGIN(APP);
__tb_app_capacity = LENGTH(APP);
__tb_app_entry = LENGTH(BOOT);
__tb_meta_base = ORIGIN(META);

/* Regions may sit between APP and META (app-owned settings); the bootloader
 * never erases or writes past LENGTH(APP). */
ASSERT(ORIGIN(APP) + LENGTH(APP) <= ORIGIN(META), "tinyboot: APP overlaps META");

/* Image tail [compat u32, version u16] in the last 6 bytes of boot flash,
 * where the app reads it. VMA set explicitly so the CODE mirror matches
 * the BOOT LMA. */
SECTIONS
{
    .tb_version (ORIGIN(CODE) + LENGTH(CODE) - 6) : AT(ORIGIN(BOOT) + LENGTH(BOOT) - 6)
    {
        KEEP(*(.tb_version));
    } > CODE
}
