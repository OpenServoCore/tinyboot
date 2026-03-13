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
        #[cfg(feature = "bootloader")]
        let flash_alias = "BOOT";
        #[cfg(all(feature = "app", not(feature = "bootloader")))]
        let flash_alias = "APP";
        #[cfg(not(any(feature = "bootloader", feature = "app")))]
        let flash_alias = {
            panic!("Select either \"bootloader\" or \"app\" feature to generate memory.x");
            #[allow(unreachable_code)]
            ""
        };

        let memory_x = gen_memory_x(BOOT_PAGES, flash_alias);
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

    #[cfg(feature = "bootloader")]
    {
        std::fs::copy("link.x", out.join("link.x"))?;
        println!("cargo:rerun-if-changed=link.x");
    }

    std::fs::copy("tiny-boot-ch32.x", out.join("tiny-boot-ch32.x"))?;

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tiny-boot-ch32.x");

    Ok(())
}


/// Certain chip specific data are stored in ch32-metapac and we
/// need to use them for constant generics. This function generates
/// the constants required for the chip.
fn gen_constants(boot_pages: usize) -> String {
    let flash = FlashInfo::new(boot_pages);
    let ram_base = METADATA
        .memory
        .iter()
        .find(|r| r.kind == MemoryRegionKind::Ram)
        .unwrap()
        .address;

    let mut s = String::new();

    // Constants needed by both boot and app
    use std::fmt::Write;
    writeln!(s, "/// Base address of the boot meta struct.").unwrap();
    writeln!(s, "pub(crate) const META_BASE: u32 = {};", flash.sections.meta.base).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "/// Base address of RAM.").unwrap();
    writeln!(s, "pub(crate) const RAM_BASE: u32 = {};", ram_base).unwrap();

    // Constants only needed by the bootloader
    #[cfg(feature = "bootloader")]
    {
        writeln!(s).unwrap();
        writeln!(s, "/// Size of a single write operation in bytes.").unwrap();
        writeln!(s, "pub(crate) const FLASH_WRITE_SIZE: usize = {};", flash.write_size).unwrap();
        writeln!(s).unwrap();
        writeln!(s, "/// Size of a single erase operation in bytes.").unwrap();
        writeln!(s, "pub(crate) const FLASH_ERASE_SIZE: usize = {};", flash.erase_size).unwrap();
        writeln!(s).unwrap();
        writeln!(s, "/// Base address of the application section.").unwrap();
        writeln!(s, "pub(crate) const APP_BASE: u32 = {};", flash.sections.app.base).unwrap();
        writeln!(s).unwrap();
        writeln!(s, "/// Size of the application section in bytes (excludes meta region).").unwrap();
        writeln!(s, "pub(crate) const APP_SIZE: usize = {};", flash.sections.app.size).unwrap();
    }

    s
}

/// Generate memory.x file for the given chip.
///
/// `flash_alias` controls which region FLASH is aliased to:
/// - `"BOOT"` for bootloader binaries (flash starts at 0x0)
/// - `"APP"` for application binaries (flash starts at APP_BASE)
///
/// stolen and modified from:
///   https://github.com/ch32-rs/ch32-data/blob/main/ch32-metapac-gen/src/lib.rs
#[cfg(feature = "memory-x")]
fn gen_memory_x(boot_pages: usize, flash_alias: &str) -> String {
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
REGION_ALIAS("FLASH", {flash_alias});

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
    meta: SectionInfo,
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

        // Boot meta struct is stored in the last FLASH_WRITE_SIZE bytes of
        // the flash. The app linker script should reserve this space.
        let meta_size = flash_write_size;
        let meta_address = app_address + (app_size - meta_size) as u32;

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
                    size: app_size - meta_size,
                },
                meta: SectionInfo {
                    base: meta_address,
                    size: meta_size,
                },
            },
        }
    }
}
