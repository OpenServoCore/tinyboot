#![no_std]

pub trait ChipHal {
    // UART
    fn uart_read_byte(&mut self) -> u8;
    fn uart_write_byte(&mut self, byte: u8);

    // Flash
    fn flash_erase_page(&mut self, addr: u32);
    fn flash_write_page(&mut self, addr: u32, data: &[u8; 64]);

    // System
    fn system_reset(&mut self) -> !;
}

pub enum BootMode {
    Bootloader,
    Application,
}

pub fn determine_boot_mode() -> BootMode {
    let meta = meta();

    if meta.magic != BOOTLOADER_MAGIC {
        if meta.checksum != NO_CHECKSUM {
            BootMode::Application
        } else if cfg!(feature = "require-app") {
            BootMode::Bootloader
        } else {
            BootMode::Application
        }
    } else {
        BootMode::Bootloader
    }
}

pub fn jump_to_application() -> ! {
    let entry_point: extern "C" fn() -> ! = unsafe { core::mem::transmute(app_addr() as usize) };
    entry_point();
}

#[repr(C)]
struct Meta {
    magic: u32,
    checksum: u32,
}

const BOOTLOADER_MAGIC: u32 = 0xB00710AD;
const NO_CHECKSUM: u32 = 0xFFFFFFFF;

extern "C" {
    static __APP_ADDR: u8;
    static __APP_SIZE: u8;
    static __META_ADDR: u8;
}

fn meta() -> &'static Meta {
    unsafe { &*(meta_addr() as *const Meta) }
}

fn app_addr() -> u32 {
    unsafe { &__APP_ADDR as *const u8 as u32 }
}

fn app_size() -> u32 {
    unsafe { &__APP_SIZE as *const u8 as u32 }
}

fn meta_addr() -> u32 {
    unsafe { &__META_ADDR as *const u8 as u32 }
}
