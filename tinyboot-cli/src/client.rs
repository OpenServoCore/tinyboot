use log::debug;
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{EraseData, Frame, MAX_PAYLOAD};
use tinyboot_protocol::{Cmd, Status};

#[derive(Debug, thiserror::Error)]
pub enum FlashError {
    #[error("transport I/O error")]
    Io,
    #[error("device returned error: {0:?}")]
    Device(Status),
    #[error("firmware too large: {size} bytes, device capacity: {capacity} bytes")]
    FirmwareTooLarge { size: u32, capacity: u32 },
    #[error("CRC mismatch: expected {expected:#06X}, got {actual:#06X}")]
    CrcMismatch { expected: u16, actual: u16 },
    #[error("invalid info response")]
    BadInfo,
}

/// Device geometry from Info response.
#[derive(Debug, Clone, Copy)]
pub struct DeviceInfo {
    pub capacity: u32,
    pub erase_size: u16,
    pub boot_version: u16,
    pub app_version: u16,
    pub mode: u16,
}

pub struct Client<T: embedded_io::Read + embedded_io::Write> {
    transport: T,
    frame: Frame,
}

impl<T: embedded_io::Read + embedded_io::Write> Client<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            frame: Frame::default(),
        }
    }

    /// Send current frame as a request and read the response.
    fn transact(&mut self) -> Result<(), FlashError> {
        self.frame.status = Status::Request;
        debug!(
            ">> {:?} addr={:#X} len={}",
            self.frame.cmd, self.frame.addr, self.frame.len
        );
        self.frame
            .send(&mut self.transport)
            .map_err(|_| FlashError::Io)?;
        let parse_status = self
            .frame
            .read(&mut self.transport)
            .map_err(|_| FlashError::Io)?;

        debug!(
            "<< {:?} status={:?} len={}",
            self.frame.cmd, self.frame.status, self.frame.len
        );
        if parse_status != Status::Ok {
            return Err(FlashError::Device(parse_status));
        }
        match self.frame.status {
            Status::Ok => Ok(()),
            status => Err(FlashError::Device(status)),
        }
    }

    /// Query device geometry.
    pub fn info(&mut self) -> Result<DeviceInfo, FlashError> {
        self.frame.cmd = Cmd::Info;
        self.frame.addr = 0;
        self.frame.len = 0;
        self.transact()?;

        if self.frame.len < 8 {
            return Err(FlashError::BadInfo);
        }

        let info = unsafe { self.frame.data.info };
        let capacity = { info.capacity };
        let erase_size = { info.erase_size };
        let boot_version = { info.boot_version };
        let app_version = { info.app_version };
        let mode = { info.mode };

        if erase_size == 0 || capacity == 0 {
            return Err(FlashError::BadInfo);
        }

        Ok(DeviceInfo {
            capacity,
            erase_size,
            boot_version,
            app_version,
            mode,
        })
    }

    /// Erase entire app region.
    pub fn erase(
        &mut self,
        on_progress: &mut dyn FnMut(u32, u32),
    ) -> Result<DeviceInfo, FlashError> {
        let info = self.info()?;
        let erase_size = info.erase_size as u32;
        let capacity = info.capacity;
        let mut addr = 0u32;
        while addr < capacity {
            self.frame.cmd = Cmd::Erase;
            self.frame.addr = addr;
            self.frame.len = 2;
            self.frame.data.erase = EraseData {
                byte_count: erase_size as u16,
            };
            self.transact()?;
            addr += erase_size;
            on_progress(addr, capacity);
        }
        Ok(info)
    }

    /// Flash firmware to device.
    ///
    /// Calls `on_progress(phase, current, total)` for progress reporting.
    pub fn flash(
        &mut self,
        firmware: &[u8],
        on_progress: &mut dyn FnMut(&str, u32, u32),
    ) -> Result<DeviceInfo, FlashError> {
        // 1. Info
        let info = self.info()?;

        let fw_size = firmware.len() as u32;
        if fw_size > info.capacity {
            return Err(FlashError::FirmwareTooLarge {
                size: fw_size,
                capacity: info.capacity,
            });
        }

        let erase_size = info.erase_size as u32;

        // 2. Erase — page by page
        let erase_total = fw_size.next_multiple_of(erase_size);
        let mut erase_addr = 0u32;
        while erase_addr < erase_total {
            self.frame.cmd = Cmd::Erase;
            self.frame.addr = erase_addr;
            self.frame.len = 2;
            self.frame.data.erase = EraseData {
                byte_count: erase_size as u16,
            };
            self.transact()?;
            erase_addr += erase_size;
            on_progress("Erasing", erase_addr, erase_total);
        }

        // 3. Write — chunk by MAX_PAYLOAD
        let total_chunks = firmware.len().div_ceil(MAX_PAYLOAD) as u32;
        let mut offset = 0usize;
        let mut chunk_idx = 0u32;
        while offset < firmware.len() {
            let end = (offset + MAX_PAYLOAD).min(firmware.len());
            let chunk = &firmware[offset..end];
            self.frame.cmd = Cmd::Write;
            self.frame.addr = offset as u32;
            self.frame.len = chunk.len() as u16;
            unsafe { self.frame.data.raw[..chunk.len()].copy_from_slice(chunk) };
            self.transact()?;
            offset = end;
            chunk_idx += 1;
            on_progress("Writing", chunk_idx, total_chunks);
        }

        // 4. Verify — CRC only covers firmware bytes (no padding)
        let expected_crc = crc16(CRC_INIT, firmware);

        self.frame.cmd = Cmd::Verify;
        self.frame.addr = fw_size;
        self.frame.len = 0;
        self.transact()?;

        let actual_crc = unsafe { self.frame.data.verify }.crc;
        if actual_crc != expected_crc {
            return Err(FlashError::CrcMismatch {
                expected: expected_crc,
                actual: actual_crc,
            });
        }

        Ok(info)
    }

    /// Reset the device. Does not wait for a response since the device resets immediately.
    /// `bootloader=true` (addr=1): enter bootloader. `bootloader=false` (addr=0): boot app.
    pub fn reset(&mut self, bootloader: bool) {
        self.frame.cmd = Cmd::Reset;
        self.frame.addr = u32::from(bootloader);
        self.frame.len = 0;
        self.frame.status = Status::Request;
        let _ = self.frame.send(&mut self.transport);
    }
}
