/* User-flash boot request word.
 *
 * Reserves 4 bytes at the start of RAM (NOLOAD, preserved across
 * soft resets). Both bootloader and app must link this script.
 * Add -Ttb-user-flash.x to linker flags. */
SECTIONS
{
    .boot_request ORIGIN(RAM) (NOLOAD) :
    {
        __boot_request = .;
        . += 4;
    } > RAM
} INSERT BEFORE .data;
