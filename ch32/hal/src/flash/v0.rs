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
    // RM: clear EOP flag (W1C) after every BUFRST, BUFLOAD, and STRT.
    FLASH.statr().write(|w| w.set_eop(true));
}

/// Unlock flash controller (KEYR + MODEKEYR).
pub fn unlock() {
    FLASH.keyr().write(|w| w.set_keyr(KEY1));
    FLASH.keyr().write(|w| w.set_keyr(KEY2));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY1));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY2));
}

/// Lock flash controller.
#[inline(always)]
pub fn lock() {
    FLASH.ctlr().write(|w| {
        w.set_lock(true);
        w.set_flock(true);
    });
}

/// Flash page size in bytes (erase and fast-write granularity).
pub const PAGE_SIZE: usize = 64;

/// Fast-write buffer load size in bytes.
const BUF_LOAD_SIZE: usize = 4;

// --- User flash (fast page erase/write) ---

/// Erase a single 64-byte page at `addr` (RM §16.4.7).
pub fn erase(addr: u32) {
    // Step 4: set FTER
    FLASH.ctlr().write(|w| w.set_page_er(true));
    // Step 5: write page address
    FLASH.addr().write(|w| w.set_addr(addr));
    // Step 6: set STRT
    FLASH.ctlr().write(|w| {
        w.set_page_er(true);
        w.set_strt(true);
    });
    // Step 7: wait BSY, clear EOP
    wait_busy();
    // Clear FTER
    FLASH.ctlr().write(|_| {});
}

/// Write `data` to flash at `addr` (RM §16.4.6).
///
/// Must not cross a page boundary. `data` length must be a multiple
/// of 4 bytes, `addr` must be 4-byte aligned.
pub fn write(addr: u32, data: &[u8]) {
    let page_base = addr & !(PAGE_SIZE as u32 - 1);
    debug_assert!(
        (addr as usize & (BUF_LOAD_SIZE - 1)) == 0,
        "write: addr not word-aligned"
    );
    debug_assert!(
        data.len().is_multiple_of(BUF_LOAD_SIZE),
        "write: len not word-aligned"
    );
    debug_assert!(
        addr + data.len() as u32 <= page_base + PAGE_SIZE as u32,
        "write: crosses page boundary"
    );

    // Step 4: set FTPG alone
    FLASH.ctlr().write(|w| w.set_page_pg(true));
    // Step 5: set BUFRST (with FTPG still set)
    FLASH.ctlr().write(|w| {
        w.set_page_pg(true);
        w.set_bufrst(true);
    });
    // Step 6: wait BSY, clear EOP
    wait_busy();

    // Steps 7-8: load words into buffer (repeat for each 4-byte chunk)
    let mut buf_addr = addr;
    let mut ptr = data.as_ptr() as *const u32;
    for _ in 0..data.len() / BUF_LOAD_SIZE {
        let word = unsafe { ptr.read() };
        unsafe { core::ptr::write_volatile(buf_addr as *mut u32, word) };
        FLASH.ctlr().write(|w| {
            w.set_page_pg(true);
            w.set_bufload(true);
        });
        wait_busy();
        buf_addr += BUF_LOAD_SIZE as u32;
        ptr = unsafe { ptr.add(1) };
    }

    // Step 10: write page address
    FLASH.addr().write(|w| w.set_addr(page_base));
    // Step 11: set STRT
    FLASH.ctlr().write(|w| {
        w.set_page_pg(true);
        w.set_strt(true);
    });
    // Step 12: wait BSY, clear EOP
    wait_busy();
    // Step 14: clear FTPG
    FLASH.ctlr().write(|_| {});
}

pub fn is_boot_mode() -> bool {
    FLASH.statr().read().boot_mode()
}

pub fn set_boot_mode(mode: bool) {
    FLASH.boot_modekeyp().write(|w| w.set_modekeyr(KEY1));
    FLASH.boot_modekeyp().write(|w| w.set_modekeyr(KEY2));
    FLASH.statr().write(|w| w.set_boot_mode(mode));
}
