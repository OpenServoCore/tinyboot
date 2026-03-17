#[cfg(pfic_rv2)]
core::arch::global_asm!(include_str!("v2.S"));

// defmt-rtt requires a critical-section implementation.
// Interrupts are never enabled in the bootloader, so acquire/release are no-ops.
#[cfg(feature = "defmt")]
mod cs {
    struct CriticalSection;

    critical_section::set_impl!(CriticalSection);

    unsafe impl critical_section::Impl for CriticalSection {
        unsafe fn acquire() -> critical_section::RawRestoreState {
            Default::default()
        }
        unsafe fn release(_state: critical_section::RawRestoreState) {}
    }
}
