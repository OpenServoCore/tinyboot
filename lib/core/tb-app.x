/* tinyboot app-side linker fragment.
 *
 * Expects memory.x to define CODE, BOOT, APP, META, and RAM regions,
 * plus REGION_ALIAS("FLASH", CODE) for qingke-rt compatibility.
 * Add -Ttb-app.x to linker flags in your application binary. */

/* Symbols derived from memory regions. */
__tb_meta_base = ORIGIN(META);
__tb_boot_version_addr = ORIGIN(BOOT) + LENGTH(BOOT) - 2;
__tb_app_capacity = LENGTH(APP);

/* App version must be the very last loadable flash content.
 * The bootloader reads it at app_base + app_size - 2.
 *
 * Capture the qingke-rt interrupt vector table (orphan section on V3+)
 * before .tb_version so it doesn't end up after the version tag. */
SECTIONS
{
    .tb_vectors ALIGN(4) :
    {
        KEEP(*(.vector_table.interrupts));
        KEEP(*(.vector_table.external_interrupts));
    } > CODE
} INSERT AFTER .data;

SECTIONS
{
    .tb_version ALIGN(2) :
    {
        __tb_version = .;
        KEEP(*(.tb_version));
    } > CODE
} INSERT AFTER .tb_vectors;

