    # =========================================================================
    # Minimal bootloader runtime for system-flash builds (QingKe V3).
    #
    # Omits .data copy, .bss zeroing, and interrupt vector table to save
    # space — critical when the bootloader must fit in 2048 bytes.
    #
    # Safe because the system-flash bootloader:
    #   - Has no initialized statics (no .data)
    #   - Has no zero-init statics (no .bss)
    #   - Never enables interrupts (polling only)
    #   - Only runs from power-on reset (SRAM is undefined anyway)
    #
    # User-flash builds should use the full qingke-rt runtime which
    # includes .data/.bss init for defmt-rtt and system-reset safety.
    # =========================================================================

    .section .init, "ax"
    .global _start
    .align 2

_start:
    .option push
    .option norelax
    la      gp, __global_pointer$
    .option pop

    la      sp, _stack_top
    j       main
