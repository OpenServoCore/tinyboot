/* tinyboot app-side linker fragment.
 *
 * Expects memory.x to define CODE, BOOT, APP, META, and RAM regions,
 * plus REGION_ALIAS("FLASH", CODE) for qingke-rt compatibility.
 * Add -Ttb-app.x to linker flags in your application binary. */

/* Symbols derived from memory regions. */
__tb_meta_base = ORIGIN(META);
__tb_boot_version_addr = ORIGIN(BOOT) + LENGTH(BOOT) - 2;
__tb_app_capacity = LENGTH(APP);

/* App version must be the very last loadable flash content: the bootloader
 * reads it at app_base + app_size - 2. qingke-rt places .data's load image
 * last, so anchor on it explicitly; the location counter after .data is a
 * RAM address. */
SECTIONS
{
    .tb_version ALIGN(LOADADDR(.data) + SIZEOF(.data), 2) :
    {
        __tb_version = .;
        KEEP(*(.tb_version));
    } > CODE
} INSERT AFTER .data;
