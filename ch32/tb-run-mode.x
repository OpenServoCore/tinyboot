/* Run-mode magic word.
 *
 * Reserves 4 bytes at the start of RAM (NOLOAD, preserved across
 * soft resets). Both bootloader and app must link this script.
 * Used by the RamRunModeCtl storage.
 * Add -Ttb-run-mode.x to linker flags. */
SECTIONS
{
    .run_mode ORIGIN(RAM) (NOLOAD) :
    {
        __tb_run_mode = .;
        . += 4;
    } > RAM
} INSERT BEFORE .data;
