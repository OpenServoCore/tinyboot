/* tinyboot app-side linker fragment.
 *
 * Expects memory.x to define CODE, BOOT, APP, META, and RAM regions,
 * plus REGION_ALIAS("FLASH", CODE) for qingke-rt compatibility.
 * Add -Ttb-app.x to linker flags in your application binary. */

/* Symbols derived from memory regions. */
__tb_meta_base = ORIGIN(META);
__tb_boot_version_addr = ORIGIN(BOOT) + LENGTH(BOOT) - 2;
__tb_app_capacity = LENGTH(APP);

/* App version placed immediately after all other flash content. */
SECTIONS
{
    .tb_version ALIGN(2) :
    {
        __tb_version = .;
        KEEP(*(.tb_version));
    } > CODE
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
