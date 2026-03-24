const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;

fn flash() -> ch32_metapac::flash::Flash {
    ch32_metapac::FLASH
}

#[inline(always)]
fn wait_busy() {
    while flash().statr().read().bsy() {}
}

/// Unlock flash controller for all operations (KEYR + MODEKEYR + OBKEYR).
pub fn unlock() {
    flash().keyr().write(|w| w.set_keyr(KEY1));
    flash().keyr().write(|w| w.set_keyr(KEY2));
    flash().modekeyr().write(|w| w.set_modekeyr(KEY1));
    flash().modekeyr().write(|w| w.set_modekeyr(KEY2));
    flash().obkeyr().write(|w| w.set_optkey(KEY1));
    flash().obkeyr().write(|w| w.set_optkey(KEY2));
}

/// Lock flash controller.
pub fn lock() {
    flash().ctlr().write(|w| {
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

const PG: u8 = 0; // PG bit - standard programming - 2b
const FTER: u8 = 17; // FTER bit - fast page erase - 64B

const OBPG: u8 = 4; // OBPG bit - option byte programming
const OBER: u8 = 5; // OBER bit - option byte erase

const OBWRE: u8 = 9; // OBWRE bit - option byte write enabled. This needs to be 1 after unlock.
const STRT: u8 = 6; // STRT bit - start operation

impl FlashWriter {
    /// Writer for regular operations (PG / FTER).
    pub const fn standard() -> Self {
        Self {
            erase_bit: FTER,
            write_bit: PG,
        }
    }

    /// Writer for option bytes (OBPG / OBER).
    pub const fn ob() -> Self {
        Self {
            erase_bit: OBER,
            write_bit: OBPG,
        }
    }

    /// check for write protection error
    pub fn check_wrprterr(&self) -> bool {
        let statr = flash().statr().read();
        if statr.wrprterr() {
            flash().statr().write(|w| w.set_wrprterr(true));
            return true;
        }
        if statr.eop() {
            flash().statr().write(|w| w.set_eop(true));
        }
        false
    }

    /// Start write operation
    #[inline(always)]
    pub fn write_start(&self) {
        let write_bit = self.write_bit as usize;
        let ctlr_start = (1 << OBWRE) | (1 << write_bit);
        flash().ctlr().write(|w| w.0 = ctlr_start);
    }

    /// Halfword (2-byte) write.
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
        flash().ctlr().write(|w| w.0 = ctlr_start);
    }

    /// Erase (64-byte page for flash, full OB erase for option bytes).
    pub fn erase(&self, addr: u32) {
        let erase_bit = self.erase_bit as usize;
        let ctlr = (1 << OBWRE) | (1 << erase_bit) | (1 << STRT);

        flash().addr().write(|w| w.set_addr(addr));
        flash().ctlr().write(|w| w.0 = ctlr);
        wait_busy();
    }

    // End write or erase operation
    #[inline(always)]
    pub fn operation_end(&self) {
        let ctlr_end = 1 << OBWRE; // preserve OB write enable bit
        flash().ctlr().write(|w| w.0 = ctlr_end);
    }
}

pub fn is_boot_mode() -> bool {
    flash().statr().read().boot_mode()
}

pub fn set_boot_mode(mode: bool) {
    flash().boot_modekeyp().write(|w| w.set_modekeyr(KEY1));
    flash().boot_modekeyp().write(|w| w.set_modekeyr(KEY2));
    flash().statr().write(|w| w.set_boot_mode(mode));
}
