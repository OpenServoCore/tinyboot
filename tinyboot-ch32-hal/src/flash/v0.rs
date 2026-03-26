const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;
const FLASH: ch32_metapac::flash::Flash = ch32_metapac::FLASH;

#[inline(always)]
fn wait_busy() {
    while FLASH.statr().read().bsy() {}
    debug_assert!(
        !FLASH.statr().read().wrprterr(),
        "flash write protection error"
    );
}

/// Unlock flash controller for all operations (KEYR + MODEKEYR + OBKEYR).
pub fn unlock() {
    FLASH.keyr().write(|w| w.set_keyr(KEY1));
    FLASH.keyr().write(|w| w.set_keyr(KEY2));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY1));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY2));
    FLASH.obkeyr().write(|w| w.set_optkey(KEY1));
    FLASH.obkeyr().write(|w| w.set_optkey(KEY2));
}

/// Lock flash controller.
#[inline(always)]
pub fn lock() {
    FLASH.ctlr().write(|w| {
        w.set_lock(true);
        w.set_flock(true);
    });
}

/// Flash/OB writer. Thin wrapper selecting CTLR bit positions.
/// Requires `unlock()` to have been called first.
pub struct FlashWriter {
    erase_bit: u8,
    write_bit: u8,
}

/// Flash page size in bytes (erase and fast-write granularity).
pub const PAGE_SIZE: usize = 64;

/// Fast-write buffer load size in bytes.
pub const BUF_LOAD_SIZE: usize = 4;

const FTPG: u8 = 16; // FTPG bit - fast page programming - 64B
const FTER: u8 = 17; // FTER bit - fast page erase - 64B
const BUFLOAD: u8 = 18; // BUFLOAD bit - fast-program buffer load
const BUFRST: u8 = 19; // BUFRST bit - fast-program buffer reset

const OBPG: u8 = 4; // OBPG bit - option byte programming
const OBER: u8 = 5; // OBER bit - option byte erase

const OBWRE: u8 = 9; // OBWRE bit - option byte write enabled. This needs to be 1 after unlock.
const STRT: u8 = 6; // STRT bit - start operation

impl FlashWriter {
    /// Writer for user flash (FTPG / FTER).
    pub const fn usr() -> Self {
        Self {
            erase_bit: FTER,
            write_bit: FTPG,
        }
    }

    /// Writer for option bytes (OBPG / OBER).
    pub const fn opt() -> Self {
        Self {
            erase_bit: OBER,
            write_bit: OBPG,
        }
    }

    /// Start write operation
    #[inline(always)]
    pub fn write_start(&self) {
        let write_bit = self.write_bit as usize;
        let ctlr_start = (1 << OBWRE) | (1 << write_bit);
        FLASH.ctlr().write(|w| w.0 = ctlr_start);
    }

    /// Halfword (2-byte) write. used only for OB writes.
    /// Use `fast_write_*` for flash writes.
    #[inline(always)]
    pub fn write(&self, addr: u32, value: u16) {
        unsafe { core::ptr::write_volatile(addr as *mut u16, value) };
        wait_busy();
    }

    /// Start erase operation
    #[inline(always)]
    pub fn erase_start(&self) {
        let erase_bit = self.erase_bit as usize;
        let ctlr_start = (1 << OBWRE) | (1 << erase_bit);
        FLASH.ctlr().write(|w| w.0 = ctlr_start);
    }

    /// Erase (64-byte page for flash, full OB erase for option bytes).
    #[inline(always)]
    pub fn erase(&self, addr: u32) {
        let erase_bit = self.erase_bit as usize;
        let ctlr = (1 << OBWRE) | (1 << erase_bit) | (1 << STRT);

        FLASH.addr().write(|w| w.set_addr(addr));
        FLASH.ctlr().write(|w| w.0 = ctlr);
        wait_busy();
    }

    // End write or erase operation
    #[inline(always)]
    pub fn operation_end(&self) {
        let ctlr_end = 1 << OBWRE; // preserve OB write enable bit
        FLASH.ctlr().write(|w| w.0 = ctlr_end);
    }

    /// Clear the internal 64-byte fast-programming buffer (BUFRST).
    pub fn fast_write_buf_reset(&self) {
        let ctlr = (1 << OBWRE) | (1 << FTPG) | (1 << BUFRST);
        FLASH.ctlr().write(|w| w.0 = ctlr);
        wait_busy();
    }

    /// Load 4 bytes into the fast-programming buffer at `addr`.
    pub fn fast_write_buf_load(&self, addr: u32, value: u32) {
        unsafe { core::ptr::write_volatile(addr as *mut u32, value) };
        let ctlr = (1 << OBWRE) | (1 << FTPG) | (1 << BUFLOAD);
        FLASH.ctlr().write(|w| w.0 = ctlr);
        wait_busy();
    }

    /// Program the buffered page to flash at `page_addr`.
    pub fn fast_write_page_program(&self, page_addr: u32) {
        FLASH.addr().write(|w| w.set_addr(page_addr));
        let ctlr = (1 << OBWRE) | (1 << FTPG) | (1 << STRT);
        FLASH.ctlr().write(|w| w.0 = ctlr);
        wait_busy();
    }
}

pub fn is_boot_mode() -> bool {
    FLASH.statr().read().boot_mode()
}

pub fn set_boot_mode(mode: bool) {
    FLASH.boot_modekeyp().write(|w| w.set_modekeyr(KEY1));
    FLASH.boot_modekeyp().write(|w| w.set_modekeyr(KEY2));
    FLASH.statr().write(|w| w.set_boot_mode(mode));
}
