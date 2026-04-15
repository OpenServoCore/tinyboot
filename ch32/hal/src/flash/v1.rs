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

/// Undocumented flash controller register at offset 0x34.
/// Not in the RM; used by WCH's HAL for read-cache invalidation.
///
/// See: https://github.com/openwch/ch32v103/blob/f99a84c4c42b6fb676560a9b8b7c737401efe0ad/EVT/EXAM/SRC/Peripheral/src/ch32v10x_flash.c#L210
const FLASH_RDCACHE_REG: *mut u32 = 0x4002_2034 as *mut u32;

/// XOR mask used to read from a 4 KB-distant flash address, forcing eviction
/// of the prefetch/read pipeline entry for the page just written or erased.
const RDCACHE_XOR_MASK: u32 = 0x0000_1000;

/// Invalidate the flash read cache for `addr`.
///
/// `addr` must be word-aligned (callers always pass page-aligned addresses).
fn invalidate_read_cache(addr: u32) {
    let src = addr ^ RDCACHE_XOR_MASK;
    let val = unsafe { core::ptr::read_volatile(src as *const u32) };
    unsafe { core::ptr::write_volatile(FLASH_RDCACHE_REG, val) };
}

/// Flash page size in bytes (erase and fast-write granularity).
pub const PAGE_SIZE: usize = 128;

/// Fast-write buffer load size in bytes.
const BUF_LOAD_SIZE: usize = 16;

// --- User flash (fast page erase/write) ---

/// Erase a single 128-byte page at `addr` (RM §24.4.7).
pub fn erase(addr: u32) {
    // Step 4: set FTER
    FLASH.ctlr().write(|w| {
        w.set_fter(true);
    });
    // Step 5: write page address
    FLASH.addr().write(|w| w.set_far(addr));
    // Step 6: set STRT
    FLASH.ctlr().write(|w| {
        w.set_fter(true);
        w.set_strt(true);
    });
    // Step 7: wait BSY, clear EOP
    wait_busy();
    // Clear FTER
    FLASH.ctlr().write(|_| {});
    invalidate_read_cache(addr);
}

/// Write `data` to flash at `addr` (RM §24.4.6).
///
/// Must not cross a page boundary. `addr` must be 4-byte aligned.
/// Trailing bytes are padded to the next BUF_LOAD_SIZE boundary with 0xFF.
pub fn write(addr: u32, data: &[u8]) {
    let page_base = addr & !(PAGE_SIZE as u32 - 1);
    debug_assert!((addr as usize & 3) == 0, "write: addr not word-aligned");
    debug_assert!(
        addr + data.len() as u32 <= page_base + PAGE_SIZE as u32,
        "write: crosses page boundary"
    );

    // Step 4: set FTPG alone
    FLASH.ctlr().write(|w| {
        w.set_ftpg(true);
    });
    // Step 5: set BUFRST (with FTPG still set)
    FLASH.ctlr().write(|w| {
        w.set_ftpg(true);
        w.set_bufrst(true);
    });
    // Step 6: wait BSY, clear EOP
    wait_busy();

    // Steps 7-9: load 16-byte groups into buffer (8 groups × 16 bytes = 128 bytes).
    // Each group: write 4 words, then BUFLOAD.
    let load_len = (data.len() + BUF_LOAD_SIZE - 1) & !(BUF_LOAD_SIZE - 1);
    let mut buf_addr = addr;
    let mut pos = 0;
    while pos < load_len {
        // Build word byte-by-byte; 0xFF fills bytes past data end.
        let mut buf = [0xFFu8; 4];
        let mut j = 0;
        while j < 4 && pos + j < data.len() {
            buf[j] = data[pos + j];
            j += 1;
        }
        unsafe { core::ptr::write_volatile(buf_addr as *mut u32, u32::from_le_bytes(buf)) };
        buf_addr += 4;
        pos += 4;

        // Step 8: BUFLOAD after each 16-byte group
        if pos % BUF_LOAD_SIZE == 0 {
            FLASH.ctlr().write(|w| {
                w.set_ftpg(true);
                w.set_bufload(true);
            });
            wait_busy();
        }
    }

    // Step 10: write page address
    FLASH.addr().write(|w| w.set_far(page_base));
    // Step 11: set STRT
    FLASH.ctlr().write(|w| {
        w.set_ftpg(true);
        w.set_strt(true);
    });
    // Step 12: wait BSY, clear EOP
    wait_busy();
    // Step 14: clear FTPG
    FLASH.ctlr().write(|_| {});
    invalidate_read_cache(page_base);
}
