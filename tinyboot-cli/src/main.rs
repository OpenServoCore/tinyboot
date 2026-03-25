mod client;
mod transport;

use std::time::Instant;

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use object::elf::{PT_LOAD, SHF_ALLOC};
use object::read::elf::{ElfFile32, ProgramHeader as _};
use object::{LittleEndian, Object, ObjectSection, SectionFlags};

use client::Client;
use transport::Serial;

#[derive(Parser)]
#[command(name = "tinyboot", about = "tinyboot firmware flasher")]
struct Cli {
    /// Increase verbosity (-v debug, -vv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args)]
struct ConnectionArgs {
    /// Serial port (e.g. /dev/ttyUSB0). Auto-detects if omitted.
    #[arg(long)]
    port: Option<String>,
    /// Baud rate
    #[arg(long, default_value_t = 115200)]
    baud: u32,
}

#[derive(Subcommand)]
enum Commands {
    /// Query device info (capacity, payload size, erase size)
    Info {
        #[command(flatten)]
        conn: ConnectionArgs,
    },
    /// Erase entire app region
    Erase {
        #[command(flatten)]
        conn: ConnectionArgs,
    },
    /// Flash firmware to device
    Flash {
        /// Firmware binary file
        firmware: String,
        #[command(flatten)]
        conn: ConnectionArgs,
        /// Reset device after flashing
        #[arg(long)]
        reset: bool,
    },
    /// Convert ELF to flat binary (same extraction as flash)
    Bin {
        /// Input ELF or binary file
        firmware: String,
        /// Output .bin file
        #[arg(short, long)]
        output: String,
    },
    /// Reset the device
    Reset {
        #[command(flatten)]
        conn: ConnectionArgs,
        /// Reset into bootloader instead of booting app
        #[arg(long)]
        bootloader: bool,
    },
}

/// Probe each available serial port for a tinyboot device (bootloader or app).
/// Sends Info — both the bootloader and apps with poll_cmd respond to it.
fn detect_port(baud: u32) -> Result<String, Box<dyn std::error::Error>> {
    let ports = serialport::available_ports()?;
    if ports.is_empty() {
        return Err("no serial ports found".into());
    }
    for p in &ports {
        let serial = match serialport::new(&p.port_name, baud)
            .timeout(std::time::Duration::from_millis(500))
            .open()
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  skipping {}: {e}", p.port_name);
                continue;
            }
        };
        let mut client = Client::new(Serial(serial));
        if client.info().is_ok() {
            eprintln!("detected tinyboot on {}", p.port_name);
            return Ok(p.port_name.clone());
        }
    }
    Err("no tinyboot device found on any serial port".into())
}

fn resolve_port(port: Option<String>, baud: u32) -> Result<String, Box<dyn std::error::Error>> {
    match port {
        Some(p) => Ok(p),
        None => detect_port(baud),
    }
}

/// Load firmware from file. If ELF, extract loadable sections into a flat binary
/// using physical addresses (LMA). Skips `.uninit.*` sections.
/// If raw binary (no ELF magic), use as-is.
///
/// CH32 flash is at 0x0800_0000 but some linker scripts map it to 0x0000_0000.
/// LMAs below 0x0800_0000 are adjusted by adding the flash base offset.
fn load_firmware(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    const FLASH_BASE: u32 = 0x0800_0000;

    if data.get(..4) != Some(b"\x7fELF") {
        return Ok(data.to_vec());
    }

    let endian = LittleEndian;
    let elf = ElfFile32::<LittleEndian>::parse(data)?;

    // Build VMA→LMA mapping from PT_LOAD segments
    let load_segs: Vec<_> = elf
        .elf_program_headers()
        .iter()
        .filter(|ph| ph.p_type(endian) == PT_LOAD)
        .collect();

    let vma_to_lma = |vma: u32| -> Option<u32> {
        for ph in &load_segs {
            let seg_vma = ph.p_vaddr(endian);
            let seg_memsz = ph.p_memsz(endian);
            if vma >= seg_vma && vma < seg_vma + seg_memsz {
                let mut lma = ph.p_paddr(endian) + (vma - seg_vma);
                if lma < FLASH_BASE {
                    lma += FLASH_BASE;
                }
                return Some(lma);
            }
        }
        None
    };

    // Collect (LMA, data) for ALLOC sections with file data, excluding .uninit.*
    let mut regions: Vec<(u32, &[u8])> = Vec::new();
    for section in elf.sections() {
        let name = section.name().unwrap_or("");
        let is_alloc = matches!(
            section.flags(),
            SectionFlags::Elf { sh_flags } if sh_flags & u64::from(SHF_ALLOC) != 0
        );
        if !is_alloc || name.starts_with(".uninit") {
            continue;
        }
        let sdata = section.data()?;
        if sdata.is_empty() {
            continue;
        }
        let vma = section.address() as u32;
        let lma = vma_to_lma(vma)
            .ok_or_else(|| format!("section '{name}' at VMA {vma:#X} not in any LOAD segment"))?;
        regions.push((lma, sdata));
    }

    if regions.is_empty() {
        return Err("ELF has no loadable sections".into());
    }

    let base = regions.iter().map(|(lma, _)| *lma).min().unwrap();
    let end = regions
        .iter()
        .map(|(lma, d)| *lma + d.len() as u32)
        .max()
        .unwrap();

    let size = (end - base) as usize;
    let mut binary = vec![0xFFu8; size];

    for (lma, sdata) in &regions {
        let offset = (*lma - base) as usize;
        binary[offset..offset + sdata.len()].copy_from_slice(sdata);
    }

    eprintln!("ELF: base {:#010X}, {} bytes", base, size);
    Ok(binary)
}

fn open_serial(port: &str, baud: u32) -> Result<Serial, Box<dyn std::error::Error>> {
    let port = serialport::new(port, baud)
        .timeout(std::time::Duration::from_secs(5))
        .open()
        .map_err(|e| {
            eprintln!("serial open error: {e:?}");
            e
        })?;
    Ok(Serial(port))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        _ => log::LevelFilter::Debug,
    };
    env_logger::Builder::new().filter_level(log_level).init();

    match cli.command {
        Commands::Info { conn } => {
            let port = resolve_port(conn.port, conn.baud)?;
            let serial = open_serial(&port, conn.baud)?;
            let mut client = Client::new(serial);
            let info = client.info()?;
            println!("capacity:     {} bytes", info.capacity);
            println!("erase_size:   {} bytes", info.erase_size);
            let (bm, bn, bp) = tinyboot_protocol::unpack_version(info.boot_version);
            let (am, an, ap) = tinyboot_protocol::unpack_version(info.app_version);
            if info.boot_version == 0xFFFF {
                println!("boot_version: none");
            } else {
                println!("boot_version: {bm}.{bn}.{bp}");
            }
            if info.app_version == 0xFFFF {
                println!("app_version:  none");
            } else {
                println!("app_version:  {am}.{an}.{ap}");
            }
            println!(
                "mode:         {}",
                if info.mode == 0 { "bootloader" } else { "app" }
            );
        }
        Commands::Erase { conn } => {
            let port = resolve_port(conn.port, conn.baud)?;
            let serial = open_serial(&port, conn.baud)?;
            let mut client = Client::new(serial);

            let info = client.info()?;
            if info.mode != 0 {
                return Err("device is running the app, not the bootloader. Run `tinyboot reset --bootloader` first.".into());
            }

            let pb = ProgressBar::new(0);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("Erasing [{bar:30}] {pos}/{len}")?
                    .progress_chars("=> "),
            );

            let info = client.erase(&mut |current, total| {
                pb.set_length(total as u64);
                pb.set_position(current as u64);
            })?;
            pb.finish_and_clear();

            println!(
                "OK — erased {} bytes ({} pages)",
                info.capacity,
                info.capacity / info.erase_size as u32
            );
        }
        Commands::Bin { firmware, output } => {
            let file_data = std::fs::read(&firmware)?;
            let fw = load_firmware(&file_data)?;
            std::fs::write(&output, &fw)?;
            println!("{} bytes written to {output}", fw.len());
        }
        Commands::Flash {
            firmware,
            conn,
            reset,
        } => {
            let port = resolve_port(conn.port, conn.baud)?;
            let file_data = std::fs::read(&firmware)?;
            let fw = load_firmware(&file_data)?;
            let serial = open_serial(&port, conn.baud)?;
            let mut client = Client::new(serial);

            let info = client.info()?;
            if info.mode != 0 {
                return Err("device is running the app, not the bootloader. Run `tinyboot reset --bootloader` first.".into());
            }

            let start = Instant::now();
            let pb = ProgressBar::new(0);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:30}] {pos}/{len}")?
                    .progress_chars("=> "),
            );

            let mut current_phase = String::new();
            let info = client.flash(&fw, &mut |phase, current, total| {
                if phase != current_phase {
                    current_phase = phase.to_string();
                    pb.set_length(total as u64);
                    pb.set_position(0);
                    pb.set_message(current_phase.clone());
                }
                pb.set_position(current as u64);
            })?;
            pb.finish_and_clear();

            let elapsed = start.elapsed();
            println!(
                "OK — {} bytes written to {} byte region in {:.1}s",
                fw.len(),
                info.capacity,
                elapsed.as_secs_f64()
            );

            if reset {
                client.reset(false); // addr=0: boot app
                println!("device reset");
            }
        }
        Commands::Reset { conn, bootloader } => {
            let port = resolve_port(conn.port, conn.baud)?;
            let serial = open_serial(&port, conn.baud)?;
            let mut client = Client::new(serial);
            client.reset(bootloader);
            println!(
                "device reset ({})",
                if bootloader { "bootloader" } else { "app" }
            );
        }
    }

    Ok(())
}
