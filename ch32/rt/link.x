INCLUDE memory.x

ENTRY(_start);

SECTIONS
{
    .text ORIGIN(CODE) :
    {
        KEEP(*(SORT_NONE(.init)));
        . = ALIGN(4);
        *(.text .text.*);
    } > CODE AT> BOOT

    .rodata : ALIGN(4)
    {
        *(.srodata .srodata.* .rodata .rodata.*);
        . = ALIGN(4);
    } > CODE AT> BOOT

    .data : ALIGN(4)
    {
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.* .sdata2 .sdata2.* .data .data.*);
        . = ALIGN(4);
    } > RAM AT> BOOT

    .bss (NOLOAD) : ALIGN(4)
    {
        *(.sbss .sbss.* .bss .bss.*);
        . = ALIGN(4);
    } > RAM

    _stack_top = ORIGIN(RAM) + LENGTH(RAM);

    .got (INFO) : { KEEP(*(.got .got.*)) }
    .eh_frame (INFO) : { KEEP(*(.eh_frame)) }
    .eh_frame_hdr (INFO) : { *(.eh_frame_hdr) }
}
