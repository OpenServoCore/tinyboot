use core::sync::atomic::{Ordering, fence};

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;

/// The FPEC on flash_v0 chips requires 0x0800_0000-based addresses for
/// programming, even though flash is mapped at 0x0000_0000 for reads.
const FLASH_PROGRAM_BASE: u32 = 0x0800_0000;

fn flash() -> ch32_metapac::flash::Flash {
    ch32_metapac::FLASH
}

fn wait_busy() {
    while flash().statr().read().bsy() {}
}

/// RAII guard for flash programming. Unlocks on creation, locks on drop.
pub(crate) struct FlashWriter;

impl FlashWriter {
    pub fn standard() -> Self {
        flash().keyr().write(|w| w.set_keyr(KEY1));
        fence(Ordering::SeqCst);
        flash().keyr().write(|w| w.set_keyr(KEY2));
        fence(Ordering::SeqCst);
        Self
    }

    pub fn fast() -> Self {
        let s = Self::standard();
        flash().modekeyr().write(|w| w.set_modekeyr(KEY1));
        fence(Ordering::SeqCst);
        flash().modekeyr().write(|w| w.set_modekeyr(KEY2));
        fence(Ordering::SeqCst);
        s
    }

    pub fn check_wrprterr(&self) -> bool {
        let statr = flash().statr().read();
        if statr.wrprterr() {
            flash().statr().modify(|w| w.set_wrprterr(true));
            return true;
        }
        if statr.eop() {
            flash().statr().modify(|w| w.set_eop(true));
        }
        false
    }

    pub fn write_halfword(&self, addr: u32, value: u16) {
        let prog_addr = FLASH_PROGRAM_BASE + addr;
        flash().ctlr().modify(|w| w.set_pg(true));
        fence(Ordering::SeqCst);
        unsafe { core::ptr::write_volatile(prog_addr as *mut u16, value) };
        wait_busy();
        flash().ctlr().modify(|w| w.set_pg(false));
    }

    pub fn erase_page(&self, addr: u32) {
        flash().ctlr().modify(|w| w.set_page_er(true));
        fence(Ordering::SeqCst);
        flash()
            .addr()
            .write(|w| w.set_addr(FLASH_PROGRAM_BASE + addr));
        fence(Ordering::SeqCst);
        flash().ctlr().modify(|w| w.set_strt(true));
        wait_busy();
        flash().ctlr().modify(|w| w.set_page_er(false));
    }

    pub fn write_page(&self, addr: u32, data: &[u8]) {
        let prog_addr = FLASH_PROGRAM_BASE + addr;

        flash().ctlr().modify(|w| w.set_page_pg(true));
        flash().ctlr().modify(|w| w.set_bufrst(true));
        wait_busy();
        flash().ctlr().modify(|w| w.set_page_pg(false));

        let mut ptr = prog_addr as *mut u32;
        for chunk in data.chunks_exact(4) {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            flash().ctlr().modify(|w| w.set_page_pg(true));
            unsafe { core::ptr::write_volatile(ptr, word) };
            flash().ctlr().modify(|w| w.set_bufload(true));
            wait_busy();
            flash().ctlr().modify(|w| w.set_page_pg(false));
            ptr = unsafe { ptr.add(1) };
        }

        flash().ctlr().modify(|w| w.set_page_pg(true));
        flash().addr().write(|w| w.set_addr(prog_addr));
        flash().ctlr().modify(|w| w.set_strt(true));
        wait_busy();
        flash().ctlr().modify(|w| w.set_page_pg(false));
    }
}

impl Drop for FlashWriter {
    fn drop(&mut self) {
        flash().ctlr().modify(|w| {
            w.set_lock(true);
            w.set_flock(true);
        });
    }
}

pub(crate) fn is_boot_mode() -> bool {
    flash().statr().read().boot_mode()
}

pub(crate) fn set_boot_mode(mode: bool) {
    if flash().statr().read().boot_lock() {
        flash().boot_modekeyp().write(|w| w.set_modekeyr(KEY1));
        fence(Ordering::SeqCst);
        flash().boot_modekeyp().write(|w| w.set_modekeyr(KEY2));
        fence(Ordering::SeqCst);
    }
    flash().statr().modify(|w| w.set_boot_mode(mode));
}
