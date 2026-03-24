use crate::ringbuf::RingBuf;
use crate::traits::boot::{BootCtl, BootMetaStore, Platform, Storage, Transport};
use crate::traits::{BootMode, BootState};
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{Frame, InfoData, VerifyData};
use tinyboot_protocol::{Cmd, ReadError, Status};

/// Protocol dispatcher with write buffering.
///
/// Incoming write data is accumulated in a ring buffer and flushed to storage
/// in page-sized chunks using fast page programming. The host must send a
/// `Flush` command to commit any remaining partial page before `Verify`,
/// or before skipping to a non-sequential address.
pub struct Dispatcher<'a, T: Transport, S: Storage, B: BootMetaStore, C: BootCtl, const BUF: usize>
{
    /// Mutable reference to the platform peripherals.
    pub platform: &'a mut Platform<T, S, B, C>,
    /// Reusable frame buffer.
    pub frame: Frame,
    /// Write buffer. Sized for 2 × page size.
    buf: RingBuf<BUF>,
    /// Expected address of the next sequential write. `None` = accept any.
    next_addr: Option<u32>,
}

impl<'a, T: Transport, S: Storage, B: BootMetaStore, C: BootCtl, const BUF: usize>
    Dispatcher<'a, T, S, B, C, BUF>
{
    /// Create a new dispatcher for the given platform.
    pub fn new(platform: &'a mut Platform<T, S, B, C>) -> Self {
        Self {
            platform,
            frame: Frame::default(),
            buf: RingBuf::default(),
            next_addr: None,
        }
    }

    /// Write `n` bytes from the buffer to storage, deriving the address from `next_addr`.
    fn write_buf(&mut self, next: u32, n: usize) {
        let addr = next - self.buf.len() as u32;
        let data = self.buf.peek(n);
        if self.platform.storage.write(addr, data).is_err() {
            self.frame.status = Status::WriteError;
        }
        self.buf.consume(n);
    }

    /// Read a frame, dispatch the command, and send the response.
    /// Returns Err only for transport IO errors. Frame-level errors
    /// (bad CRC, invalid frame) are silently skipped.
    pub fn dispatch(&mut self) -> Result<(), ReadError> {
        let status = self.frame.read(&mut self.platform.transport)?;

        if status != Status::Ok {
            self.frame.len = 0;
            self.frame.status = status;
            return self
                .frame
                .send(&mut self.platform.transport)
                .map_err(|_| ReadError);
        }

        let data_len = self.frame.len as usize;
        let capacity = self.platform.storage.capacity() as u32;
        let erase_size = S::ERASE_SIZE as u32;
        let write_size = S::WRITE_SIZE as u32;
        let state = self.platform.boot_meta.boot_state();
        self.frame.len = 0;
        self.frame.status = Status::Ok;

        match self.frame.cmd {
            Cmd::Info => {
                self.frame.len = 12;
                let app_sz = self.platform.boot_meta.app_size();
                let app_ver = if app_sz != 0xFFFF_FFFF {
                    // SAFETY: app_size != 0xFFFFFFFF means meta was previously written
                    // by a Verify that validated app_size against capacity.
                    let base = self.platform.storage.as_slice().as_ptr();
                    unsafe { base.add(app_sz as usize - 2).cast::<u16>().read_volatile() }
                } else {
                    0xFFFF
                };
                self.frame.data.info = InfoData {
                    capacity,
                    erase_size: erase_size as u16,
                    boot_version: self.platform.boot_version,
                    app_version: app_ver,
                    mode: 0,
                };
            }
            Cmd::Erase => {
                let addr = self.frame.addr;
                let byte_count = unsafe { self.frame.data.erase }.byte_count as u32;
                if !addr.is_multiple_of(erase_size)
                    || !byte_count.is_multiple_of(erase_size)
                    || byte_count == 0
                    || addr + byte_count > capacity
                {
                    self.frame.status = Status::AddrOutOfBounds;
                } else {
                    // State transitions for erase
                    match state {
                        // Idle → Updating: step down state byte
                        BootState::Idle => {
                            if self.platform.boot_meta.advance().is_err() {
                                self.frame.status = Status::WriteError;
                            }
                        }
                        // Validating → Updating: app failed, reflashing
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
                        // Updating → Updating: no state change
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
            }
            Cmd::Write => {
                if state != BootState::Updating {
                    self.frame.status = Status::Unsupported;
                } else {
                    let addr = self.frame.addr;
                    if addr + data_len as u32 > capacity
                        || !addr.is_multiple_of(write_size)
                        || self.next_addr.is_some_and(|n| n != addr)
                    {
                        self.frame.status = Status::AddrOutOfBounds;
                    } else {
                        // SAFETY: data_len <= MAX_PAYLOAD validated by frame.read()
                        self.buf
                            .push(unsafe { self.frame.data.raw.get_unchecked(..data_len) });
                        let next = addr + data_len as u32;
                        self.next_addr = Some(next);
                        // Flush full page
                        if self.buf.len() >= S::WRITE_SIZE {
                            self.write_buf(next, S::WRITE_SIZE);
                        }
                    }
                }
            }
            Cmd::Verify => {
                if state != BootState::Updating {
                    self.frame.status = Status::Unsupported;
                } else {
                    let app_size = self.frame.addr;
                    let sz = app_size as usize;
                    if sz == 0 || sz > capacity as usize {
                        self.frame.status = Status::AddrOutOfBounds;
                    } else {
                        // SAFETY: sz bounds-checked against capacity above.
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
                }
            }
            Cmd::Reset => {
                let _ = self.frame.send(&mut self.platform.transport);
                let mode = if self.frame.addr == 1 {
                    BootMode::Bootloader
                } else {
                    BootMode::App
                };
                self.platform.ctl.system_reset(mode);
            }
            Cmd::Flush => {
                if let Some(next) = self.next_addr {
                    if !self.buf.is_empty() {
                        self.write_buf(next, self.buf.len());
                    }
                    self.next_addr = None;
                }
            }
        }

        self.frame
            .send(&mut self.platform.transport)
            .map_err(|_| ReadError)
    }
}
