use std::env;
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use ch32_metapac::metadata::{METADATA, MemoryRegionKind};

const BOOT_PAGES: usize = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    #[cfg(feature = "memory-x")]
    {
        let memory_x = gen_memory_x(BOOT_PAGES);
        File::create(out.join("memory.x"))?.write_all(memory_x.as_bytes())?;
    }

    let constants = gen_constants(BOOT_PAGES);
    File::create(out.join("constants.rs"))?.write_all(constants.as_bytes())?;

    // emit peripheral variant cfgs
    for p in METADATA.peripherals {
        if let Some(regs) = &p.registers {
            let cfg = format!("{}_{}", regs.kind, regs.version);
            println!("cargo::rustc-check-cfg=cfg({cfg})");
            println!("cargo:rustc-cfg={cfg}");
        }
    }

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

/// Certain chip specific data are stored in ch32-metapac and we
/// need to use them for constant generics. This function generates
/// the constants required for the chip.
fn gen_constants(boot_pages: usize) -> String {
    let flash = FlashInfo::new(boot_pages);

    format!(
        r#"
/// Base address of the flash memory.
pub(crate) const FLASH_BASE: u32 = {};

/// Size of the flash memory in bytes.
pub(crate) const FLASH_SIZE: usize = {};

/// Size of a single write operation in bytes.
pub(crate) const FLASH_WRITE_SIZE: usize = {};

/// Size of a single erase operation in bytes.
pub(crate) const FLASH_ERASE_SIZE: usize = {};

/// Base address of the boot section.
pub(crate) const BOOT_BASE: u32 = {};

/// Size of the boot section in bytes.
pub(crate) const BOOT_SIZE: usize = {};

/// Base address of the application section.
pub(crate) const APP_BASE: u32 = {};

/// Size of the application section in bytes.
pub(crate) const APP_SIZE: usize = {};
"#,
        flash.base,
        flash.size,
        flash.write_size,
        flash.erase_size,
        flash.sections.boot.base,
        flash.sections.boot.size,
        flash.sections.app.base,
        flash.sections.app.size,
    )
}

/// Generate memory.x file for the given chip.
/// stolen and modified from:
///   https://github.com/ch32-rs/ch32-data/blob/main/ch32-metapac-gen/src/lib.rs
#[cfg(feature = "memory-x")]
fn gen_memory_x(boot_pages: usize) -> String {
    let mut memory_x = String::new();

    let flash = FlashInfo::new(boot_pages);

    let ram = METADATA
        .memory
        .iter()
        .find(|r| r.kind == MemoryRegionKind::Ram)
        .unwrap();
    let otp = METADATA
        .memory
        .iter()
        .find(|r| r.kind == MemoryRegionKind::Flash && r.name == "OTP");

    write!(memory_x, "MEMORY\n{{\n").unwrap();
    writeln!(
        memory_x,
        "    BOOT : ORIGIN = 0x{:08x}, LENGTH = {:>4}",
        flash.sections.boot.base, flash.sections.boot.size
    )
    .unwrap();
    writeln!(
        memory_x,
        "    APP  : ORIGIN = 0x{:08x}, LENGTH = {:>4}K - {:>4}",
        flash.sections.app.base,
        flash.size / 1024,
        flash.sections.boot.size
    )
    .unwrap();
    writeln!(
        memory_x,
        "    RAM   : ORIGIN = 0x{:08x}, LENGTH = {:>4}K",
        ram.address,
        ram.size / 1024,
    )
    .unwrap();
    if let Some(otp) = otp {
        writeln!(
            memory_x,
            "    OTP   : ORIGIN = 0x{:08x}, LENGTH = {:>4}",
            otp.address, otp.size,
        )
        .unwrap();
    }
    write!(memory_x, "}}").unwrap();

    write!(
        memory_x,
        r#"
# Change BOOT to APP in your application.
REGION_ALIAS("FLASH", BOOT);

# Defines the usual regions using FLASH
REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
    "#
    )
    .unwrap();

    memory_x
}

struct FlashInfo {
    base: u32,
    size: usize,
    erase_size: usize,
    write_size: usize,
    sections: Sections,
}

struct Sections {
    boot: SectionInfo,
    app: SectionInfo,
}

struct SectionInfo {
    base: u32,
    size: usize,
}

impl FlashInfo {
    fn new(boot_pages: usize) -> FlashInfo {
        let (flash_base, flash_size, flash_erase_size, flash_write_size) = METADATA
            .memory
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Flash && r.name.starts_with("BANK_"))
            .map(|r| {
                (
                    r.address,
                    r.size as usize,
                    r.settings.as_ref().map_or(0, |s| s.erase_size) as usize,
                    r.settings.as_ref().map_or(0, |s| s.write_size) as usize,
                )
            })
            .reduce(|acc, el| {
                (
                    // smallest address is the beginning of the flash
                    u32::min(acc.0, el.0),
                    // total size is the sum of all flash regions
                    acc.1 + el.1,
                    // a safe erase size for all flash banks is the biggest erase size
                    usize::max(acc.2, el.2),
                    // a safe write size for all flash banks is the biggest write size
                    usize::max(acc.3, el.3),
                )
            })
            .unwrap();

        let boot_address = flash_base;
        let boot_size = boot_pages * flash_erase_size;
        let app_address = boot_address + boot_size as u32;
        let app_size = flash_size - boot_size;

        FlashInfo {
            base: flash_base,
            size: flash_size,
            erase_size: flash_erase_size,
            write_size: flash_write_size,
            sections: Sections {
                boot: SectionInfo {
                    base: boot_address,
                    size: boot_size,
                },
                app: SectionInfo {
                    base: app_address,
                    size: app_size,
                },
            },
        }
    }
}
