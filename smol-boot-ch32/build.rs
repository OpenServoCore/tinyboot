use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    #[cfg(feature = "memory-x")]
    {
        let memory_x = gen_memory_x(4);
        File::create(out.join("memory.x"))?.write_all(memory_x.as_bytes())?;
    }

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

/// Generate memory.x file for the given chip.
/// stolen and modified from:
///   https://github.com/ch32-rs/ch32-data/blob/main/ch32-metapac-gen/src/lib.rs
#[cfg(feature = "memory-x")]
fn gen_memory_x(boot_pages: u32) -> String {
    use ch32_metapac::metadata::{METADATA, MemoryRegionKind};
    use std::fmt::Write as _;
    let mut memory_x = String::new();

    let flash = METADATA
        .memory
        .iter()
        .filter(|r| r.kind == MemoryRegionKind::Flash && r.name.starts_with("BANK_"));
    let (flash_address, flash_size, erase_size) = flash
        .map(|r| {
            (
                r.address,
                r.size,
                r.settings.as_ref().map_or(0, |s| s.erase_size),
            )
        })
        .reduce(|acc, el| (u32::min(acc.0, el.0), acc.1 + el.1, u32::max(acc.2, el.2)))
        .unwrap();
    let ram = METADATA
        .memory
        .iter()
        .find(|r| r.kind == MemoryRegionKind::Ram)
        .unwrap();
    let otp = METADATA
        .memory
        .iter()
        .find(|r| r.kind == MemoryRegionKind::Flash && r.name == "OTP");

    // compute bootloader and app address / size
    let boot_size = boot_pages * erase_size;
    let boot_address = flash_address;
    let app_address = boot_address + boot_size;

    write!(memory_x, "MEMORY\n{{\n").unwrap();
    writeln!(
        memory_x,
        "    BOOT : ORIGIN = 0x{:08x}, LENGTH = {:>4}",
        boot_address, boot_size,
    )
    .unwrap();
    writeln!(
        memory_x,
        "    APP  : ORIGIN = 0x{:08x}, LENGTH = {:>4}K - {:>4}",
        app_address,
        flash_size / 1024,
        boot_size,
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

# External Constants used in bootloader
__APP_ADDR  = ORIGIN(APP);
__APP_SIZE = LENGTH(APP);
    "#
    )
    .unwrap();

    memory_x
}
