INCLUDE memory.x

ENTRY(_start);

SECTIONS
{
    .vector_table ORIGIN(CODE) :
    {
        KEEP(*(SORT_NONE(.init)));
        . = ALIGN(4);
        KEEP(*(.vector_table.core_interrupts));
        KEEP(*(.vector_table.external_interrupts));
        KEEP(*(.vector_table.exceptions));
        *(.trap .trap.rust);
    } > CODE AT> FLASH

    .text : ALIGN(4)
    {
        KEEP(*(SORT_NONE(.handle_reset)));
        *(.init.rust);
        *(.text .text.*);
    } > CODE AT> FLASH

    .rodata : ALIGN(4)
    {
        *(.srodata .srodata.*);
        *(.rodata .rodata.*);
        . = ALIGN(4);
    } > CODE AT> FLASH

    .data : ALIGN(4)
    {
        _sidata = LOADADDR(.data);
        _sdata = .;
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.* .sdata2 .sdata2.*);
        *(.data .data.*);
        . = ALIGN(4);
        _edata = .;
    } > RAM AT> FLASH

    .bss (NOLOAD) : ALIGN(4)
    {
        _sbss = .;
        *(.sbss .sbss.* .bss .bss.*);
        . = ALIGN(4);
        _ebss = .;
    } > RAM

    .uninit (NOLOAD) : ALIGN(4)
    {
        *(.uninit .uninit.*);
    } > RAM

    _stack_top = ORIGIN(RAM) + LENGTH(RAM);

    .got (INFO) : { KEEP(*(.got .got.*)) }
    .eh_frame (INFO) : { KEEP(*(.eh_frame)) }
    .eh_frame_hdr (INFO) : { *(.eh_frame_hdr) }
}
