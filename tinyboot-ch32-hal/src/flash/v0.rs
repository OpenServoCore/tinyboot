use tinyboot_macros::tb_assert;

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;
const FLASH: ch32_metapac::flash::Flash = ch32_metapac::FLASH;

#[inline(always)]
fn wait_busy() {
    while FLASH.statr().read().bsy() {}
    tb_assert!(
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

/// Flash page size in bytes (erase and fast-write granularity).
pub const PAGE_SIZE: usize = 64;

/// Fast-write buffer load size in bytes.
const BUF_LOAD_SIZE: usize = 4;

// --- User flash (fast page erase/write) ---

/// Erase a single 64-byte page at `addr`.
pub fn usr_erase(addr: u32) {
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_page_er(true);
    });
    FLASH.addr().write(|w| w.set_addr(addr));
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_page_er(true);
        w.set_strt(true);
    });
    wait_busy();
}

/// Write `data` to flash at `addr`. Must not cross a page boundary.
/// `data` length must be a multiple of 4 bytes, `addr` must be 4-byte aligned.
pub fn usr_write(addr: u32, data: &[u8]) {
    let page_base = addr & !(PAGE_SIZE as u32 - 1);
    tb_assert!(
        (addr as usize & (BUF_LOAD_SIZE - 1)) == 0,
        "usr_write: addr not word-aligned"
    );
    tb_assert!(
        data.len().is_multiple_of(BUF_LOAD_SIZE),
        "usr_write: len not word-aligned"
    );
    tb_assert!(
        addr + data.len() as u32 <= page_base + PAGE_SIZE as u32,
        "usr_write: crosses page boundary"
    );
    // FTPG mode + buf reset
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_page_pg(true);
        w.set_bufrst(true);
    });
    wait_busy();

    // Load words into buffer
    let mut buf_addr = addr;
    let mut ptr = data.as_ptr() as *const u32;
    for _ in 0..data.len() / BUF_LOAD_SIZE {
        // SAFETY: ptr advances within data bounds; caller ensures 4-byte alignment.
        let word = unsafe { ptr.read() };
        unsafe { core::ptr::write_volatile(buf_addr as *mut u32, word) };
        FLASH.ctlr().write(|w| {
            w.set_obwre(true);
            w.set_page_pg(true);
            w.set_bufload(true);
        });
        wait_busy();
        buf_addr += BUF_LOAD_SIZE as u32;
        ptr = unsafe { ptr.add(1) };
    }

    // Program the page
    FLASH.addr().write(|w| w.set_addr(page_base));
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_page_pg(true);
        w.set_strt(true);
    });
    wait_busy();
}

// --- Option bytes (standard 2-byte erase/write) ---

/// Erase all option bytes.
pub fn ob_erase() {
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_ober(true);
    });
    FLASH.addr().write(|w| w.set_addr(super::OB_BASE));
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_ober(true);
        w.set_strt(true);
    });
    wait_busy();
}

/// Write option bytes starting at `addr`. Must not cross a page boundary.
/// Each byte in `data` is written as a halfword at stride-2 addresses
/// (hardware auto-generates complement bytes).
pub fn ob_write(addr: u32, data: &[u8]) {
    tb_assert!(
        (addr as usize & 1) == 0,
        "ob_write: addr not halfword-aligned"
    );
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_obpg(true);
    });
    let mut ob_addr = addr;
    for &byte in data {
        unsafe { core::ptr::write_volatile(ob_addr as *mut u16, byte as u16) };
        wait_busy();
        ob_addr += 2;
    }
}

pub fn boot_mode() -> bool {
    FLASH.statr().read().boot_mode()
}

pub fn set_boot_mode(mode: bool) {
    FLASH.boot_modekeyp().write(|w| w.set_modekeyr(KEY1));
    FLASH.boot_modekeyp().write(|w| w.set_modekeyr(KEY2));
    FLASH.statr().write(|w| w.set_boot_mode(mode));
}
