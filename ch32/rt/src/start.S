    # Minimal bootloader runtime: no .data/.bss init, no vector table.
    # Safe because the bootloader has no mutable statics, no interrupts,
    # and only runs from power-on reset. Apps must use qingke-rt instead.

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
