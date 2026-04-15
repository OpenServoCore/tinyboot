/* Boot request magic word.
 *
 * Reserves 4 bytes at the start of RAM (NOLOAD, preserved across
 * soft resets). Both bootloader and app must link this script.
 * Used by the ram and gpio boot control schemes.
 * Add -Ttb-boot-req.x to linker flags. */
SECTIONS
{
    .boot_request ORIGIN(RAM) (NOLOAD) :
    {
        __tb_boot_request = .;
        . += 4;
    } > RAM
} INSERT BEFORE .data;
