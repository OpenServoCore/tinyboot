//! App-side tinyboot client. Handles boot confirmation and responds to
//! Info/Reset commands so the CLI can drive updates without physical access.

use crate::traits::{BootCtl, BootMetaStore, BootState, RunMode};
use tinyboot_protocol::frame::{Frame, InfoData};
use tinyboot_protocol::{Cmd, Status};

/// App-side configuration.
pub struct AppConfig {
    /// App region capacity in bytes.
    pub capacity: u32,
    /// Erase page size in bytes.
    pub erase_size: u16,
    /// Boot version (caller reads it from flash).
    pub boot_version: u16,
    /// App version, typically `pkg_version!()`.
    pub app_version: u16,
}

/// App-side tinyboot client.
pub struct App<C: BootCtl, M: BootMetaStore> {
    frame: Frame,
    config: AppConfig,
    ctl: C,
    meta: M,
}

impl<C: BootCtl, M: BootMetaStore> App<C, M> {
    /// Create a new app client.
    pub fn new(config: AppConfig, ctl: C, meta: M) -> Self {
        Self {
            frame: Frame::default(),
            config,
            ctl,
            meta,
        }
    }

    /// Validating → Idle. Runs in a critical section; feed the watchdog first.
    pub fn confirm(&mut self) {
        critical_section::with(|_| {
            if self.meta.boot_state() != BootState::Validating {
                return;
            }
            let checksum = self.meta.app_checksum();
            let app_size = self.meta.app_size();
            let _ = self.meta.refresh(checksum, BootState::Idle, app_size);
        });
    }

    /// Poll for one command (blocking).
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

    /// Poll for one command (async).
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
                    self.ctl.set_run_mode(RunMode::Service);
                }
                self.ctl.reset();
            }
            _ => {
                self.frame.len = 0;
                self.frame.status = Status::Unsupported;
            }
        }
    }
}
