unsafe extern "C" {
    static __APP_ADDR: u8;
    static __APP_SIZE: u8;
}

// Application Magic located at the beginning of the application's binary.
const APP_MAGIC: u32 = 0xC0FF_EEEE;

// Application origin address from linker symbol.
fn app_addr() -> *const u32 {
    &raw const __APP_ADDR as *const u32
}

// Application flash area size
fn app_size() -> usize {
    &raw const __APP_SIZE as usize
}

// Utilize Ch32's Optional User Data DATA0 to store boot request flag.
// This is a 16-bit register, so we need to read both bytes with
// upper as inverse, and lower as data to ensure data integrity,
// thus the u16 pointer.
const OB_DATA0: *const u16 = 0x1FFFF804 as *const u16;

#[derive(PartialEq)]
enum AppState {
    Valid,
    Invalid,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum BootMode {
    Application = 0x00,
    Bootloader = 0x01,
}

pub struct BootControl {
    app_state: AppState,
    mode: Option<BootMode>,
}

impl AppState {
    fn read() -> Self {
        let first_word = unsafe { core::ptr::read_volatile(app_addr()) };

        if first_word == APP_MAGIC {
            AppState::Valid
        } else {
            AppState::Invalid
        }
    }
}

impl BootMode {
    fn read() -> Option<Self> {
        read_ob(OB_DATA0).and_then(|b| match b {
            0x00 => Some(BootMode::Application),
            0x01 => Some(BootMode::Bootloader),
            _ => None,
        })
    }
}

impl BootControl {
    pub fn read() -> Self {
        Self {
            app_state: AppState::read(),
            mode: BootMode::read(),
        }
    }

    pub fn should_boot_app(&self) -> bool {
        if self.app_state == AppState::Invalid {
            return false;
        }

        match self.mode {
            None => true, // fresh chip, valid app, go for it
            Some(BootMode::Bootloader) => false,
            Some(BootMode::Application) => true,
        }
    }

    /// Jump to the application. Does not return.
    pub unsafe fn jump_to_app(&self) -> ! {
        unsafe {
            let entry = app_addr().add(1); // skip past 4-byte magic word
            let entry: unsafe extern "C" fn() -> ! = core::mem::transmute(entry);
            entry()
        }
    }
}

/// Read the u16 pointer from the Optional User Data register via MMIO.
/// If upper byte is not the inverse of the lower byte, return None to represent invalid data.
/// Otherwise, return the lower byte as value.
fn read_ob(addr: *const u16) -> Option<u8> {
    let raw = unsafe { core::ptr::read_volatile(addr) };
    let data = raw as u8;
    let inv = (raw >> 8) as u8;
    (data == !inv).then_some(data)
}
