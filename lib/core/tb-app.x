/* tinyboot app-side linker fragment.
 *
 * Expects memory.x to define CODE, BOOT, APP, META, and RAM regions,
 * plus REGION_ALIAS("FLASH", CODE) for qingke-rt compatibility.
 * Add -Ttb-app.x to linker flags in your application binary. */

/* Symbols derived from memory regions. */
__tb_meta_base = ORIGIN(META);
__tb_boot_tail = ORIGIN(BOOT) + LENGTH(BOOT) - 6;
__tb_app_capacity = LENGTH(APP);

/* The image tail [compat u32, version u16] must be the very last loadable
 * flash content: the bootloader reads it back from app_base + app_size.
 * It starts word-aligned so the key is a word load. qingke-rt places
 * .data's load image last, so anchor on it explicitly; the location
 * counter after .data is a RAM address. */
SECTIONS
{
    .tb_version ALIGN(LOADADDR(.data) + SIZEOF(.data), 4) :
    {
        KEEP(*(.tb_version));
    } > CODE
} INSERT AFTER .data;
