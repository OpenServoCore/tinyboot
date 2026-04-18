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

fn unlock() {
    FLASH.keyr().write(|w| w.set_keyr(KEY1));
    FLASH.keyr().write(|w| w.set_keyr(KEY2));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY1));
    FLASH.modekeyr().write(|w| w.set_modekeyr(KEY2));
}

#[inline(always)]
fn lock() {
    FLASH.ctlr().write(|w| {
        w.set_lock(true);
        w.set_flock(true);
    });
}

/// Undocumented read-cache register at offset 0x34. Not in the RM but used by
/// WCH's HAL; see openwch/ch32v103 EVT `ch32v10x_flash.c`.
const FLASH_RDCACHE_REG: *mut u32 = 0x4002_2034 as *mut u32;

/// Reading 4 KB away evicts the pipeline entry for the touched page.
const RDCACHE_XOR_MASK: u32 = 0x0000_1000;

fn invalidate_read_cache(addr: u32) {
    let src = addr ^ RDCACHE_XOR_MASK;
    let val = unsafe { core::ptr::read_volatile(src as *const u32) };
    unsafe { core::ptr::write_volatile(FLASH_RDCACHE_REG, val) };
}

pub const PAGE_SIZE: usize = 128;
const BUF_LOAD_SIZE: usize = 16;

/// Erase one 128-byte page (RM §24.4.7).
pub fn erase(addr: u32) {
    unlock();
    FLASH.ctlr().write(|w| {
        w.set_fter(true);
    });
    FLASH.addr().write(|w| w.set_far(addr));
    FLASH.ctlr().write(|w| {
        w.set_fter(true);
        w.set_strt(true);
    });
    wait_busy();
    FLASH.ctlr().write(|_| {});
    invalidate_read_cache(addr);
    lock();
}

/// Fast-page write (RM §24.4.6). `addr` 4-byte aligned, no page crossing.
/// Trailing bytes are 0xFF-padded to the next 16-byte group.
pub fn write(addr: u32, data: &[u8]) {
    let page_base = addr & !(PAGE_SIZE as u32 - 1);
    debug_assert!((addr as usize & 3) == 0, "write: addr not word-aligned");
    debug_assert!(
        addr + data.len() as u32 <= page_base + PAGE_SIZE as u32,
        "write: crosses page boundary"
    );

    unlock();
    FLASH.ctlr().write(|w| {
        w.set_ftpg(true);
    });
    FLASH.ctlr().write(|w| {
        w.set_ftpg(true);
        w.set_bufrst(true);
    });
    wait_busy();

    // Load 16-byte groups (8 groups × 16 B = 128 B page). BUFLOAD each group.
    let load_len = (data.len() + BUF_LOAD_SIZE - 1) & !(BUF_LOAD_SIZE - 1);
    let mut buf_addr = addr;
    let mut pos = 0;
    while pos < load_len {
        let mut buf = [0xFFu8; 4];
        let mut j = 0;
        while j < 4 && pos + j < data.len() {
            buf[j] = data[pos + j];
            j += 1;
        }
        unsafe { core::ptr::write_volatile(buf_addr as *mut u32, u32::from_le_bytes(buf)) };
        buf_addr += 4;
        pos += 4;

        if pos % BUF_LOAD_SIZE == 0 {
            FLASH.ctlr().write(|w| {
                w.set_ftpg(true);
                w.set_bufload(true);
            });
            wait_busy();
        }
    }

    FLASH.addr().write(|w| w.set_far(page_base));
    FLASH.ctlr().write(|w| {
        w.set_ftpg(true);
        w.set_strt(true);
    });
    wait_busy();
    FLASH.ctlr().write(|_| {});
    invalidate_read_cache(page_base);
    lock();
}
