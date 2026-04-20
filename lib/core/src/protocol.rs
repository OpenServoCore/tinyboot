use crate::platform::Platform;
use crate::ringbuf::RingBuf;
use crate::traits::{BootCtl, BootMetaStore, BootState, RunMode, Storage, Transport};
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{Frame, InfoData, VerifyData};
use tinyboot_protocol::{Cmd, ReadError, ResetFlags, Status, WriteFlags};

/// Protocol dispatcher with write buffering.
///
/// Writes are accumulated in a ring buffer and flushed a page at a time.
/// The host sets [`WriteFlags::FLUSH`] on the last write of each contiguous
/// region to commit the trailing partial page.
pub struct Dispatcher<'a, T: Transport, S: Storage, B: BootMetaStore, C: BootCtl, const BUF: usize>
{
    /// Platform peripherals.
    pub platform: &'a mut Platform<T, S, B, C>,
    /// Reusable frame buffer.
    pub frame: Frame,
    /// 2 × page size.
    buf: RingBuf<BUF>,
    /// Expected address of the next sequential write; `None` accepts any.
    next_addr: Option<u32>,
}

impl<'a, T: Transport, S: Storage, B: BootMetaStore, C: BootCtl, const BUF: usize>
    Dispatcher<'a, T, S, B, C, BUF>
{
    /// Create a new dispatcher over the given platform.
    pub fn new(platform: &'a mut Platform<T, S, B, C>) -> Self {
        Self {
            platform,
            frame: Frame::default(),
            buf: RingBuf::default(),
            next_addr: None,
        }
    }

    fn write_buf(&mut self, next: u32, n: usize) {
        let addr = next - self.buf.len() as u32;
        let data = self.buf.peek(n);
        if self.platform.storage.write(addr, data).is_err() {
            self.frame.status = Status::WriteError;
        }
        self.buf.consume(n);
    }

    #[inline(always)]
    fn cmd_write(
        &mut self,
        addr: u32,
        data_len: usize,
        flags: WriteFlags,
        capacity: u32,
        state: BootState,
    ) {
        if state != BootState::Updating {
            self.frame.status = Status::Unsupported;
            return;
        }

        let flush = flags.contains(WriteFlags::FLUSH);

        if addr + data_len as u32 > capacity
            || (self.next_addr.is_none() && !addr.is_multiple_of(S::WRITE_SIZE as u32))
            || self.next_addr.is_some_and(|n| n != addr)
        {
            self.frame.status = Status::AddrOutOfBounds;
            return;
        }

        // SAFETY: data_len <= MAX_PAYLOAD validated by frame.read()
        self.buf
            .push(unsafe { self.frame.data.raw.get_unchecked(..data_len) });
        let next = addr + data_len as u32;
        self.next_addr = Some(next);
        if self.buf.len() >= S::WRITE_SIZE {
            self.write_buf(next, S::WRITE_SIZE);
        }
        if flush {
            if !self.buf.is_empty() {
                self.write_buf(next, self.buf.len());
            }
            self.buf.reset();
            self.next_addr = None;
        }
    }

    #[inline(always)]
    fn cmd_erase(&mut self, addr: u32, capacity: u32, state: BootState) {
        let erase_size = S::ERASE_SIZE as u32;
        let byte_count = unsafe { self.frame.data.erase }.byte_count as u32;

        if !addr.is_multiple_of(erase_size)
            || !byte_count.is_multiple_of(erase_size)
            || byte_count == 0
            || addr + byte_count > capacity
        {
            self.frame.status = Status::AddrOutOfBounds;
            return;
        }

        match state {
            BootState::Idle => {
                if self.platform.boot_meta.advance().is_err() {
                    self.frame.status = Status::WriteError;
                }
            }
            BootState::Validating => {
                if self
                    .platform
                    .boot_meta
                    .refresh(0xFFFF, BootState::Updating, 0xFFFF_FFFF)
                    .is_err()
                {
                    self.frame.status = Status::WriteError;
                }
            }
            BootState::Updating => {}
        }

        if self.frame.status == Status::Ok
            && self
                .platform
                .storage
                .erase(addr, addr + byte_count)
                .is_err()
        {
            self.frame.status = Status::WriteError;
        }
    }

    #[inline(always)]
    fn cmd_info(&mut self, capacity: u32) {
        let app_sz = self.platform.boot_meta.app_size();
        let app_ver = if app_sz != 0xFFFF_FFFF {
            // SAFETY: non-default app_size implies Verify bounded it.
            let base = self.platform.storage.as_slice().as_ptr();
            unsafe { base.add(app_sz as usize - 2).cast::<u16>().read_volatile() }
        } else {
            0xFFFF
        };
        self.frame.len = 12;
        self.frame.data.info = InfoData {
            capacity,
            erase_size: S::ERASE_SIZE as u16,
            boot_version: crate::tinyboot_version(),
            app_version: app_ver,
            mode: 0,
        };
    }

    #[inline(always)]
    fn cmd_verify(&mut self, addr: u32, capacity: u32, state: BootState) {
        if state != BootState::Updating {
            self.frame.status = Status::Unsupported;
            return;
        }

        let app_size = addr;
        let sz = app_size as usize;
        if sz == 0 || sz > capacity as usize {
            self.frame.status = Status::AddrOutOfBounds;
            return;
        }

        // SAFETY: sz bounded against capacity above.
        let crc = crc16(CRC_INIT, unsafe {
            self.platform.storage.as_slice().get_unchecked(..sz)
        });
        self.frame.len = 2;
        self.frame.data.verify = VerifyData { crc };
        if self
            .platform
            .boot_meta
            .refresh(crc, BootState::Validating, app_size)
            .is_err()
        {
            self.frame.status = Status::WriteError;
        }
    }

    #[inline(always)]
    fn cmd_reset(&mut self, flags: ResetFlags) {
        let _ = self.frame.send(&mut self.platform.transport);
        let mode = if flags.contains(ResetFlags::BOOTLOADER) {
            RunMode::Service
        } else {
            RunMode::HandOff
        };
        self.platform.ctl.set_run_mode(mode);
        self.platform.ctl.reset();
    }

    /// Read a frame, dispatch, send the response. Err only on transport IO;
    /// frame-level errors (bad CRC) are silently skipped.
    pub fn dispatch(&mut self) -> Result<(), ReadError> {
        let status = self.frame.read(&mut self.platform.transport)?;

        if status == Status::Ok {
            let data_len = self.frame.len as usize;
            let addr = self.frame.addr();
            let flags = self.frame.flags;
            let capacity = self.platform.storage.capacity() as u32;
            let state = self.platform.boot_meta.boot_state();
            self.frame.len = 0;
            self.frame.status = Status::Ok;

            let cmd = self.frame.cmd;
            if cmd == Cmd::Write {
                // SAFETY: cmd is Write, so flags.write is the active variant.
                self.cmd_write(addr, data_len, unsafe { flags.write }, capacity, state);
            } else if cmd == Cmd::Erase {
                self.cmd_erase(addr, capacity, state);
            } else if cmd == Cmd::Info {
                self.cmd_info(capacity);
            } else if cmd == Cmd::Verify {
                self.cmd_verify(addr, capacity, state);
            } else {
                // SAFETY: cmd is Reset, so flags.reset is the active variant.
                self.cmd_reset(unsafe { flags.reset });
            }
        } else {
            self.frame.len = 0;
            self.frame.status = status;
        }

        self.frame
            .send(&mut self.platform.transport)
            .map_err(|_| ReadError)?;

        embedded_io::Write::flush(&mut self.platform.transport).map_err(|_| ReadError)
    }
}
