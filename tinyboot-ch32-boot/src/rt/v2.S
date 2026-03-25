    # =========================================================================
    # Minimal bootloader runtime for system-flash builds.
    #
    # Omits .data copy, .bss zeroing, and interrupt vector table to save
    # ~68 bytes — critical when the bootloader must fit in 1920 bytes.
    #
    # Safe because the system-flash bootloader:
    #   - Has no initialized statics (no .data)
    #   - Has no zero-init statics (no .bss)
    #   - Never enables interrupts (polling only)
    #   - Only runs from power-on reset (SRAM is undefined anyway)
    #
    # User-flash builds should use v2_full.S (selected via defmt feature)
    # which includes .data/.bss init for defmt-rtt and system-reset safety.
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
