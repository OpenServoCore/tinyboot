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
    // Clear EOP flag (W1C) — V103 RM requires this
    // after every BUFRST, BUFLOAD, and STRT operation.
    FLASH.statr().write(|w| w.set_eop(true));
}

/// Unlock flash controller for all operations (KEYR + MODEKEYR + OBKEYR).
pub fn unlock() {
    FLASH.keyr().write(|w| w.set_keyr(KEY1));
    FLASH.keyr().write(|w| w.set_keyr(KEY2));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY1));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY2));
    FLASH.obkeyr().write(|w| w.set_obkeyr(KEY1));
    FLASH.obkeyr().write(|w| w.set_obkeyr(KEY2));
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
pub const PAGE_SIZE: usize = 128;

/// Fast-write buffer load size in bytes.
const BUF_LOAD_SIZE: usize = 16;

// --- User flash (fast page erase/write) ---

/// Erase a single 128-byte page at `addr`.
pub fn usr_erase(addr: u32) {
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_fter(true);
    });
    FLASH.addr().write(|w| w.set_far(addr));
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_fter(true);
        w.set_strt(true);
    });
    wait_busy();
}

/// Write `data` to flash at `addr`. Must not cross a page boundary.
/// `addr` must be 4-byte aligned. Trailing bytes are padded to the
/// next BUF_LOAD_SIZE boundary with 0xFF internally.
pub fn usr_write(addr: u32, data: &[u8]) {
    let page_base = addr & !(PAGE_SIZE as u32 - 1);
    tb_assert!((addr as usize & 3) == 0, "usr_write: addr not word-aligned");
    tb_assert!(
        addr + data.len() as u32 <= page_base + PAGE_SIZE as u32,
        "usr_write: crosses page boundary"
    );

    // RM §24.4.6: FTPG must be set alone, then BUFRST separately.
    // BUFRST clears the 128-byte page buffer to 0xFF.
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_ftpg(true);
    });
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_ftpg(true);
        w.set_bufrst(true);
    });
    wait_busy();

    // Fill the page buffer in BUF_LOAD_SIZE (16-byte) groups.
    // Each group: write 4 words to the flash address space (the write
    // address sets the position within the page), then BUFLOAD commits
    // the group into the page buffer. Repeat for each group.
    let load_len = (data.len() + BUF_LOAD_SIZE - 1) & !(BUF_LOAD_SIZE - 1);
    let mut addr = addr;
    let mut pos = 0;
    while pos < load_len {
        // Build word byte-by-byte; 0xFF fills any bytes past data end.
        let mut buf = [0xFFu8; 4];
        let mut j = 0;
        while j < 4 && pos + j < data.len() {
            buf[j] = data[pos + j];
            j += 1;
        }
        unsafe { core::ptr::write_volatile(addr as *mut u32, u32::from_le_bytes(buf)) };
        addr += 4;
        pos += 4;

        // Commit this 16-byte group to the page buffer.
        if pos % BUF_LOAD_SIZE == 0 {
            FLASH.ctlr().write(|w| {
                w.set_obwre(true);
                w.set_ftpg(true);
                w.set_bufload(true);
            });
            wait_busy();
        }
    }

    // Program the entire page buffer to flash.
    FLASH.addr().write(|w| w.set_far(page_base));
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_ftpg(true);
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
    FLASH.addr().write(|w| w.set_far(super::OB_BASE));
    FLASH.ctlr().write(|w| {
        w.set_obwre(true);
        w.set_ober(true);
        w.set_strt(true);
    });
    wait_busy();
}

/// Write option bytes starting at `addr`.
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
