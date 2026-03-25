/* Place app version at the last 2 bytes of the FLASH region.
 * The bootloader reads it at storage[app_size - 2]. */
SECTIONS
{
    .tinyboot_version ORIGIN(FLASH) + LENGTH(FLASH) - 2 :
    {
        KEEP(*(.tinyboot_version));
    } > FLASH
} INSERT AFTER .data;
