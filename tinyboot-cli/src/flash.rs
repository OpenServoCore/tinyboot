use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::Frame;
use tinyboot_protocol::{Cmd, Status};

/// Max payload buffer on the host side. Large enough for any transport.
const MAX_PAYLOAD: usize = 256;

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
    pub payload_size: u16,
    pub erase_size: u16,
}

pub struct FlashClient<T: embedded_io::Read + embedded_io::Write> {
    transport: T,
    frame: Frame<MAX_PAYLOAD>,
}

impl<T: embedded_io::Read + embedded_io::Write> FlashClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            frame: Frame::default(),
        }
    }

    /// Send current frame as a request and read the response.
    fn transact(&mut self) -> Result<(), FlashError> {
        self.frame.status = Status::Request;
        self.frame
            .send(&mut self.transport)
            .map_err(|_| FlashError::Io)?;
        self.frame
            .read(&mut self.transport)
            .map_err(|_| FlashError::Io)?;

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
        let payload_size = { info.payload_size };
        let erase_size = { info.erase_size };

        if payload_size == 0 || erase_size == 0 || capacity == 0 {
            return Err(FlashError::BadInfo);
        }

        Ok(DeviceInfo {
            capacity,
            payload_size,
            erase_size,
        })
    }

    /// Erase entire app region.
    pub fn erase(
        &mut self,
        on_progress: &mut dyn FnMut(u32, u32),
    ) -> Result<DeviceInfo, FlashError> {
        let info = self.info()?;
        let erase_size = info.erase_size as u32;
        let pages = info.capacity.div_ceil(erase_size);
        for i in 0..pages {
            self.frame.cmd = Cmd::Erase;
            self.frame.addr = i * erase_size;
            self.frame.len = 0;
            self.transact()?;
            on_progress(i + 1, pages);
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
        let payload_size = info.payload_size as usize;

        // 2. Erase — page by page
        let pages = fw_size.div_ceil(erase_size);
        for i in 0..pages {
            let addr = i * erase_size;
            self.frame.cmd = Cmd::Erase;
            self.frame.addr = addr;
            self.frame.len = 0;
            self.transact()?;
            on_progress("Erasing", i + 1, pages);
        }

        // 3. Write — chunk by payload_size
        let total_chunks = firmware.len().div_ceil(payload_size) as u32;
        let mut offset = 0usize;
        let mut chunk_idx = 0u32;
        while offset < firmware.len() {
            let end = (offset + payload_size).min(firmware.len());
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

        // 4. Verify — compute local CRC over firmware + 0xFF padding to capacity
        let mut expected_crc = crc16(CRC_INIT, firmware);
        let pad_len = info.capacity as usize - firmware.len();
        // Process padding in chunks to avoid large alloc
        let pad_buf = [0xFFu8; 256];
        let mut remaining = pad_len;
        while remaining > 0 {
            let n = remaining.min(pad_buf.len());
            expected_crc = crc16(expected_crc, &pad_buf[..n]);
            remaining -= n;
        }

        self.frame.cmd = Cmd::Verify;
        self.frame.addr = 0;
        self.frame.len = 0;
        self.transact()?;

        let actual_crc = unsafe { self.frame.data.verify }.crc;
        if actual_crc != expected_crc {
            return Err(FlashError::CrcMismatch {
                expected: expected_crc,
                actual: actual_crc,
            });
        }

        // 5. Reset — tolerate timeout since device resets immediately
        self.frame.cmd = Cmd::Reset;
        self.frame.addr = 0;
        self.frame.len = 0;
        self.frame.status = Status::Request;
        let _ = self.frame.send(&mut self.transport);
        // Don't wait for response — device is resetting

        Ok(info)
    }
}
