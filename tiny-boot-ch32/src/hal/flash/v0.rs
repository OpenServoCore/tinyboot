use core::sync::atomic::{Ordering, fence};

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;

/// The FPEC on flash_v0 chips requires 0x0800_0000-based addresses for
/// programming, even though flash is mapped at 0x0000_0000 for reads.
const FLASH_PROGRAM_BASE: u32 = 0x0800_0000;

/// RAII guard for flash programming. Unlocks on creation, locks on drop.
pub(crate) struct FlashWriter<'a> {
    regs: &'a ch32_metapac::flash::Flash,
}

impl<'a> FlashWriter<'a> {
    /// Unlock standard programming mode (KEYR only).
    /// Enables half-word writes and page erase.
    pub fn standard(regs: &'a ch32_metapac::flash::Flash) -> Self {
        regs.keyr().write(|w| w.set_keyr(KEY1));
        fence(Ordering::SeqCst);
        regs.keyr().write(|w| w.set_keyr(KEY2));
        fence(Ordering::SeqCst);
        Self { regs }
    }

    /// Unlock both standard and fast programming modes (KEYR + MODEKEYR).
    /// Enables half-word writes, page erase, and fast page programming.
    pub fn fast(regs: &'a ch32_metapac::flash::Flash) -> Self {
        let s = Self::standard(regs);
        regs.modekeyr().write(|w| w.set_modekeyr(KEY1));
        fence(Ordering::SeqCst);
        regs.modekeyr().write(|w| w.set_modekeyr(KEY2));
        fence(Ordering::SeqCst);
        s
    }

    fn wait_busy(&self) {
        while self.regs.statr().read().bsy() {}
    }

    /// Returns true if a write protection error occurred.
    pub fn check_wrprterr(&self) -> bool {
        let statr = self.regs.statr().read();
        if statr.wrprterr() {
            self.regs.statr().modify(|w| w.set_wrprterr(true));
            return true;
        }
        if statr.eop() {
            self.regs.statr().modify(|w| w.set_eop(true));
        }
        false
    }

    /// Write a single half-word (u16) using standard programming mode.
    /// `addr` must be half-word aligned.
    pub fn write_halfword(&self, addr: u32, value: u16) {
        let prog_addr = FLASH_PROGRAM_BASE + addr;
        self.regs.ctlr().modify(|w| w.set_pg(true));
        fence(Ordering::SeqCst);
        unsafe { core::ptr::write_volatile(prog_addr as *mut u16, value) };
        self.wait_busy();
        self.regs.ctlr().modify(|w| w.set_pg(false));
    }

    /// Erase a single 1KB flash page.
    pub fn erase_page(&self, addr: u32) {
        self.regs.ctlr().modify(|w| w.set_page_er(true));
        fence(Ordering::SeqCst);
        self.regs
            .addr()
            .write(|w| w.set_addr(FLASH_PROGRAM_BASE + addr));
        fence(Ordering::SeqCst);
        self.regs.ctlr().modify(|w| w.set_strt(true));
        self.wait_busy();
        self.regs.ctlr().modify(|w| w.set_page_er(false));
    }

    /// Write up to 64 bytes using fast page programming (FTPG).
    /// `addr` is the absolute flash address. `data` length must be a multiple of 4.
    /// Requires `fast()` unlock.
    pub fn write_page(&self, addr: u32, data: &[u8]) {
        let prog_addr = FLASH_PROGRAM_BASE + addr;

        // Buffer reset
        self.regs.ctlr().modify(|w| w.set_page_pg(true));
        self.regs.ctlr().modify(|w| w.set_bufrst(true));
        self.wait_busy();
        self.regs.ctlr().modify(|w| w.set_page_pg(false));

        // Load words into the page buffer
        let mut ptr = prog_addr as *mut u32;
        for chunk in data.chunks_exact(4) {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            self.regs.ctlr().modify(|w| w.set_page_pg(true));
            unsafe { core::ptr::write_volatile(ptr, word) };
            self.regs.ctlr().modify(|w| w.set_bufload(true));
            self.wait_busy();
            self.regs.ctlr().modify(|w| w.set_page_pg(false));
            ptr = unsafe { ptr.add(1) };
        }

        // Commit: set address and start programming
        self.regs.ctlr().modify(|w| w.set_page_pg(true));
        self.regs.addr().write(|w| w.set_addr(prog_addr));
        self.regs.ctlr().modify(|w| w.set_strt(true));
        self.wait_busy();
        self.regs.ctlr().modify(|w| w.set_page_pg(false));
    }
}

impl Drop for FlashWriter<'_> {
    fn drop(&mut self) {
        self.regs.ctlr().modify(|w| {
            w.set_lock(true);
            w.set_flock(true);
        });
    }
}
