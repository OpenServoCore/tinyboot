//! Bootloader example for CH32 microcontrollers.
//!
//! Two flash modes available via feature flags:
//!
//! **system-flash**: Runs from the system flash region, leaving all
//! user flash for the application. No defmt (size constrained).
//!
//! **user-flash**: Occupies first 8KB of user flash, with the application in
//! the remaining space. defmt enabled for debugging.

#![no_std]
#![no_main]

#[cfg(feature = "system-flash")]
use panic_halt as _;

#[cfg(feature = "user-flash")]
use defmt_rtt as _;

tinyboot_ch32_boot::boot_version!();

#[cfg(feature = "user-flash")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panic!");
    loop {}
}

#[cfg(all(
    feature = "system-flash",
    any(
        feature = "ch32v103c6t6",
        feature = "ch32v103c8t6",
        feature = "ch32v103c8u6",
        feature = "ch32v103r8t6",
    )
))]
use tinyboot_ch32_boot::Pin;
use tinyboot_ch32_boot::{
    BaudRate, BootCtl, BootCtlConfig, BootMetaStore, Duplex, Platform, Pull, Storage,
    StorageConfig, Usart, UsartConfig, UsartMapping,
};

// ── Application geometry ─────────────────────────────────────────────
// Adjust APP_SIZE for your chip's user flash capacity.

#[cfg(feature = "system-flash")]
const APP_BASE: u32 = 0x0800_0000;

#[cfg(feature = "user-flash")]
const APP_BASE: u32 = 0x0800_2000;
#[cfg(feature = "user-flash")]
const APP_ENTRY: u32 = 0x0000_2000;

// CH32V003: 16KB user flash
#[cfg(any(
    feature = "ch32v003f4p6",
    feature = "ch32v003a4m6",
    feature = "ch32v003f4u6",
    feature = "ch32v003j4m6",
))]
const APP_SIZE: usize = if cfg!(feature = "system-flash") {
    16 * 1024
} else {
    8 * 1024
};

// CH32V103C6: 32KB user flash
#[cfg(feature = "ch32v103c6t6")]
const APP_SIZE: usize = if cfg!(feature = "system-flash") {
    32 * 1024
} else {
    24 * 1024
};

// CH32V103C8/R8: 64KB user flash
#[cfg(any(
    feature = "ch32v103c8t6",
    feature = "ch32v103c8u6",
    feature = "ch32v103r8t6",
))]
const APP_SIZE: usize = if cfg!(feature = "system-flash") {
    64 * 1024
} else {
    56 * 1024
};

#[unsafe(export_name = "main")]
fn main() -> ! {
    // USART transport for firmware updates.
    // Select mapping and pclk for your chip.
    let transport = Usart::new(&UsartConfig {
        duplex: Duplex::Full,
        baud: BaudRate::B115200,
        pclk: 8_000_000,
        mapping: UsartMapping::Usart1Remap0,
        rx_pull: Pull::None,
        tx_en: None,
    });

    let storage = Storage::new(StorageConfig {
        app_base: APP_BASE,
        app_size: APP_SIZE,
    });
    let boot_meta = BootMetaStore::default();

    // CH32V003 (no boot pin): unit config
    #[cfg(all(
        feature = "system-flash",
        any(
            feature = "ch32v003f4p6",
            feature = "ch32v003a4m6",
            feature = "ch32v003f4u6",
            feature = "ch32v003j4m6",
        )
    ))]
    let ctl = BootCtl::new(BootCtlConfig);

    // CH32V103 (boot pin): configure GPIO pin driving BOOT0 circuit
    #[cfg(all(
        feature = "system-flash",
        any(
            feature = "ch32v103c6t6",
            feature = "ch32v103c8t6",
            feature = "ch32v103c8u6",
            feature = "ch32v103r8t6",
        )
    ))]
    let ctl = BootCtl::new(BootCtlConfig {
        pin: Pin::PA0,     // adjust to your BOOT0 control pin
        active_high: true, // RC circuit: HIGH = system flash
    });

    #[cfg(feature = "user-flash")]
    let ctl = BootCtl::new(BootCtlConfig, APP_ENTRY);

    let platform = Platform::new(transport, storage, boot_meta, ctl);
    tinyboot_ch32_boot::run(platform);
}
