//! App-side tinyboot client. Handles boot confirmation and responds to
//! Info/Reset commands so the CLI can drive updates without physical access.

use crate::addressing::{self, Addressing, Unaddressed};
use crate::traits::{BootCtl, BootMetaStore, BootState, RunMode};
use tinyboot_protocol::frame::{DEFAULT_PAYLOAD, Frame, InfoData};
use tinyboot_protocol::{BaudRate, Cmd, ResetFlags, Status};

/// App-side configuration.
pub struct AppConfig {
    /// App region capacity in bytes.
    pub capacity: u32,
    /// Erase page size in bytes.
    pub erase_size: u16,
    /// Boot version (caller reads it from the boot image's tail).
    pub boot_version: u16,
    /// Boot compatibility key (caller reads it from the boot image's tail).
    pub boot_compat: u32,
    /// App version, typically `pkg_version!()`.
    pub app_version: u16,
}

/// App-side tinyboot client. `P` is the frame's data capacity, `A` its
/// bus addressing.
pub struct App<
    C: BootCtl,
    M: BootMetaStore,
    const P: usize = DEFAULT_PAYLOAD,
    A: Addressing = Unaddressed,
> {
    frame: Frame<P>,
    config: AppConfig,
    ctl: C,
    meta: M,
    addressing: A,
    wait_bytes: fn(u8),
}

impl<C: BootCtl, M: BootMetaStore, const P: usize> App<C, M, P, Unaddressed> {
    /// Create an unaddressed app client.
    pub fn new(config: AppConfig, ctl: C, meta: M) -> Self {
        Self::with_addressing(config, ctl, meta, Unaddressed, |_| {})
    }
}

impl<C: BootCtl, M: BootMetaStore, const P: usize, A: Addressing> App<C, M, P, A> {
    /// Create an app client with the given bus addressing. `wait_bytes(n)`
    /// waits about `n` byte-times at the bus baud before a `Select`
    /// reply (see [`Transport::wait_bytes`](crate::traits::Transport::wait_bytes)).
    pub fn with_addressing(
        config: AppConfig,
        ctl: C,
        meta: M,
        addressing: A,
        wait_bytes: fn(u8),
    ) -> Self {
        Self {
            frame: Frame::default(),
            config,
            ctl,
            meta,
            addressing,
            wait_bytes,
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

    /// Bootloader baud stored in boot metadata; `None` means the
    /// bootloader's build default.
    pub fn baud(&self) -> Option<BaudRate> {
        BaudRate::from_u8(self.meta.baud())
    }

    /// Store the bootloader baud; the bootloader uses it from the next
    /// reset. Skips the flash write when it is already stored. Runs in a
    /// critical section; feed the watchdog first.
    pub fn set_baud(&mut self, baud: BaudRate) {
        critical_section::with(|_| {
            if self.meta.baud() != baud as u8 {
                let _ = self.meta.set_baud(baud as u8);
            }
        });
    }

    /// Poll for one command (blocking).
    pub fn poll<R: embedded_io::Read, W: embedded_io::Write>(&mut self, rx: &mut R, tx: &mut W) {
        let status = match self.frame.read(rx) {
            Ok(s) => s,
            Err(_) => return,
        };
        if self.process(status) {
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
        if self.process(status) {
            let _ = self.frame.send_async(tx).await;
            let _ = tx.flush().await;
        }
    }

    /// Act on a read frame; returns whether to reply. Same addressing
    /// rules as the bootloader's dispatcher.
    fn process(&mut self, status: Status) -> bool {
        if A::ADDRESSABLE {
            if status != Status::Ok || self.frame.status != Status::Request {
                return false;
            }
            if self.frame.cmd == Cmd::Select {
                return addressing::select(&mut self.addressing, &mut self.frame, self.wait_bytes);
            }
            if !self.addressing.selected() {
                return false;
            }
        } else if status != Status::Ok {
            self.frame.len = 0;
            self.frame.status = status;
            return true;
        }
        self.handle_cmd();
        self.frame.cmd != Cmd::Reset
    }

    fn handle_cmd(&mut self) {
        self.frame.status = Status::Ok;
        match self.frame.cmd {
            Cmd::Info => {
                self.frame.len = InfoData::SIZE;
                self.frame.data.info = InfoData {
                    capacity: self.config.capacity,
                    erase_size: self.config.erase_size,
                    boot_version: self.config.boot_version,
                    app_version: self.config.app_version,
                    mode: 1, // app
                    boot_baud: self.meta.baud(),
                    compat: self.config.boot_compat,
                    max_payload: P as u16,
                };
            }
            Cmd::Reset => {
                let flags = unsafe { self.frame.flags.reset };
                if flags.contains(ResetFlags::BAUD) {
                    let _ = self.meta.set_baud(self.frame.addr() as u8);
                }
                if flags.contains(ResetFlags::BOOTLOADER) {
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
