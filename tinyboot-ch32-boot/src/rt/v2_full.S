    # =========================================================================
    # Full bootloader runtime for user-flash builds with defmt.
    #
    # Includes .data copy and .bss zeroing required by defmt-rtt, and safe
    # for re-entry via system reset (SRAM retains previous app state).
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

    # --- Copy .data (flash → RAM) ---
    la      a0, _sidata
    la      a1, _sdata
    la      a2, _edata
    bgeu    a1, a2, 2f
1:
    lw      t0, (a0)
    sw      t0, (a1)
    addi    a0, a0, 4
    addi    a1, a1, 4
    bltu    a1, a2, 1b
2:

    # --- Zero .bss ---
    la      a0, _sbss
    la      a1, _ebss
    bgeu    a0, a1, 2f
1:
    sw      zero, (a0)
    addi    a0, a0, 4
    bltu    a0, a1, 1b
2:

    j       main
