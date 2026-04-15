#[cfg(pfic_rv2)]
core::arch::global_asm!(include_str!("v2.S"));

#[cfg(pfic_rv3)]
core::arch::global_asm!(include_str!("v3.S"));
