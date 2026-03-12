use core::sync::atomic::{Ordering, fence};

use super::FlashError;
use crate::hal::common::FLASH_WRITE_SIZE;

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;

/// The FPEC on flash_v0 chips requires 0x0800_0000-based addresses for
/// programming, even though flash is mapped at 0x0000_0000 for reads.
const FLASH_PROGRAM_BASE: u32 = 0x0800_0000;

pub(crate) fn unlock(regs: &ch32_metapac::flash::Flash) {
    regs.keyr().write(|w| w.set_keyr(KEY1));
    fence(Ordering::SeqCst);
    regs.keyr().write(|w| w.set_keyr(KEY2));
    fence(Ordering::SeqCst);

    regs.modekeyr().write(|w| w.set_modekeyr(KEY1));
    fence(Ordering::SeqCst);
    regs.modekeyr().write(|w| w.set_modekeyr(KEY2));
    fence(Ordering::SeqCst);
}

pub(crate) fn lock(regs: &ch32_metapac::flash::Flash) {
    regs.ctlr().modify(|w| {
        w.set_lock(true);
        w.set_flock(true);
    });
}

fn wait_busy(regs: &ch32_metapac::flash::Flash) {
    while regs.statr().read().bsy() {}
}

fn check_error(regs: &ch32_metapac::flash::Flash) -> Result<(), FlashError> {
    let statr = regs.statr().read();
    if statr.wrprterr() {
        regs.statr().modify(|w| w.set_wrprterr(true));
        return Err(FlashError::Protected);
    }
    if statr.eop() {
        regs.statr().modify(|w| w.set_eop(true));
    }
    Ok(())
}

pub(crate) fn erase_page(
    regs: &ch32_metapac::flash::Flash,
    addr: u32,
) -> Result<(), FlashError> {
    regs.ctlr().modify(|w| w.set_page_er(true));
    fence(Ordering::SeqCst);
    regs.addr().write(|w| w.set_addr(FLASH_PROGRAM_BASE + addr));
    fence(Ordering::SeqCst);
    regs.ctlr().modify(|w| w.set_strt(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_page_er(false));
    check_error(regs)
}

/// Write a single FLASH_WRITE_SIZE (64-byte) page using fast page programming.
/// `addr` must be absolute and FLASH_WRITE_SIZE-aligned.
/// `data` must be exactly FLASH_WRITE_SIZE bytes.
///
/// Sequence follows WCH's official IAP implementation:
/// PAGE_PG (FTPG) is toggled on/off around each sub-step.
pub(crate) fn write_page(
    regs: &ch32_metapac::flash::Flash,
    addr: u32,
    data: &[u8],
) -> Result<(), FlashError> {
    debug_assert_eq!(data.len(), FLASH_WRITE_SIZE);

    let prog_addr = FLASH_PROGRAM_BASE + addr;

    // Buffer reset
    regs.ctlr().modify(|w| w.set_page_pg(true));
    regs.ctlr().modify(|w| w.set_bufrst(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_page_pg(false));

    // Load 16 words into the page buffer
    let mut ptr = prog_addr as *mut u32;
    for chunk in data.chunks_exact(4) {
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        regs.ctlr().modify(|w| w.set_page_pg(true));
        unsafe { core::ptr::write_volatile(ptr, word) };
        regs.ctlr().modify(|w| w.set_bufload(true));
        wait_busy(regs);
        regs.ctlr().modify(|w| w.set_page_pg(false));
        ptr = unsafe { ptr.add(1) };
    }

    // Commit: set address and start programming
    regs.ctlr().modify(|w| w.set_page_pg(true));
    regs.addr().write(|w| w.set_addr(prog_addr));
    regs.ctlr().modify(|w| w.set_strt(true));
    wait_busy(regs);
    regs.ctlr().modify(|w| w.set_page_pg(false));

    check_error(regs)
}
