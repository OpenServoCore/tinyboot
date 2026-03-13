/* Boot request flag — 4 bytes reserved at the start of RAM.
 *
 * The app writes a magic value here and soft-resets to request bootloader entry.
 * NOLOAD ensures startup code does not zero these bytes, preserving the value
 * across soft reset.
 *
 * This section is placed before .data in RAM via INSERT BEFORE, so it works
 * with any base linker script (link.x, qingke-rt, etc). */
SECTIONS
{
    .boot_request (NOLOAD) :
    {
        . += 4;
    } > RAM
} INSERT BEFORE .data
