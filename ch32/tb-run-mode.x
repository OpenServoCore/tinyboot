/* Run-mode magic word: 4 NOLOAD bytes at RAM origin, preserved across
 * soft resets. Both bootloader and app must link this via -Ttb-run-mode.x. */
SECTIONS
{
    .run_mode ORIGIN(RAM) (NOLOAD) :
    {
        __tb_run_mode = .;
        . += 4;
    } > RAM
} INSERT BEFORE .data;
