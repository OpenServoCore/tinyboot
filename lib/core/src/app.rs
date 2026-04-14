//! App-side tinyboot client.
//!
//! Handles boot confirmation and responds to host commands (Info, Reset)
//! so the CLI can query and reset the device without physical access.

use crate::traits::app::BootClient;
use tinyboot_protocol::frame::{Frame, InfoData};
use tinyboot_protocol::{Cmd, Status};

/// App-side configuration.
pub struct AppConfig {
    /// App region capacity in bytes.
    pub capacity: u32,
    /// Erase page size in bytes.
    pub erase_size: u16,
    /// Boot version (read from flash by the caller).
    pub boot_version: u16,
    /// App version (typically from `pkg_version!()`).
    pub app_version: u16,
}

/// App-side tinyboot client. Handles Info/Reset commands and boot confirmation.
pub struct App<B: BootClient> {
    frame: Frame,
    config: AppConfig,
    client: B,
}

impl<B: BootClient> App<B> {
    /// Create a new app client.
    pub fn new(config: AppConfig, client: B) -> Self {
        Self {
            frame: Frame::default(),
            config,
            client,
        }
    }

    /// Confirm boot — transitions Validating to Idle, preserving checksum.
    /// Call after all peripherals are initialized.
    pub fn confirm(&mut self) {
        self.client.confirm();
    }

    /// Poll for tinyboot commands (blocking).
    pub fn poll<R: embedded_io::Read, W: embedded_io::Write>(&mut self, rx: &mut R, tx: &mut W) {
        let status = match self.frame.read(rx) {
            Ok(s) => s,
            Err(_) => return,
        };
        if status == Status::Ok {
            self.handle_cmd();
        } else {
            self.frame.len = 0;
            self.frame.status = status;
        }
        if self.frame.cmd != Cmd::Reset {
            let _ = self.frame.send(tx);
            let _ = tx.flush();
        }
    }

    /// Poll for tinyboot commands (async).
    pub async fn poll_async<R: embedded_io_async::Read, W: embedded_io_async::Write>(
        &mut self,
        rx: &mut R,
        tx: &mut W,
    ) {
        let status = match self.frame.read_async(rx).await {
            Ok(s) => s,
            Err(_) => return,
        };
        if status == Status::Ok {
            self.handle_cmd();
        } else {
            self.frame.len = 0;
            self.frame.status = status;
        }
        if self.frame.cmd != Cmd::Reset {
            let _ = self.frame.send_async(tx).await;
            let _ = tx.flush().await;
        }
    }

    fn handle_cmd(&mut self) {
        self.frame.status = Status::Ok;
        match self.frame.cmd {
            Cmd::Info => {
                self.frame.len = 12;
                self.frame.data.info = InfoData {
                    capacity: self.config.capacity,
                    erase_size: self.config.erase_size,
                    boot_version: self.config.boot_version,
                    app_version: self.config.app_version,
                    mode: 1, // app
                };
            }
            Cmd::Reset => {
                if self.frame.addr == 1 {
                    self.client.request_update();
                }
                self.client.system_reset();
            }
            _ => {
                self.frame.len = 0;
                self.frame.status = Status::Unsupported;
            }
        }
    }
}
