#![no_std]

//! Minimal bootloader runtime for CH32 — tiny `_start` + `link.x`, no
//! `.data`/`.bss` init. For bootloader binaries that must fit in ~2 KB.
//! Apps should keep `qingke-rt`; do not link both or `_start` collides.

core::arch::global_asm!(include_str!("start.S"));
