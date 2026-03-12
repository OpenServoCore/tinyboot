#![no_std]

pub mod traits;

mod log;

use traits::{BootCtl, BootState, BootStateStore, Platform, Storage, Transport};

pub struct Core<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootStateStore,
    C: BootCtl,
{
    platform: Platform<T, S, B, C>,
}

impl<T, S, B, C> Core<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootStateStore,
    C: BootCtl,
{
    pub fn new(platform: Platform<T, S, B, C>) -> Self {
        Core { platform }
    }

    pub fn run(&mut self) -> ! {
        log_info!("Bootloader started");

        let state = self.platform.boot_state.state().unwrap_or(BootState::Idle);
        log_info!("Boot state: {:?}", state);

        match state {
            BootState::Idle => self.handle_idle(),
            BootState::Updating => self.handle_updating(),
            BootState::Validating => self.handle_validating(),
            BootState::Confirmed => self.handle_confirmed(),
        }
    }

    /// Check if the app region contains valid code by reading the first word.
    /// Erased flash reads as 0xFFFFFFFF.
    fn app_is_blank(&mut self) -> bool {
        let mut buf = [0u8; 4];
        if self.platform.storage.read(0, &mut buf).is_err() {
            return true;
        }
        buf == [0xFF; 4]
    }

    fn try_boot_app(&mut self) -> ! {
        if self.app_is_blank() {
            log_info!("No valid application found, entering bootloader mode");
            self.enter_bootloader();
        }
        self.platform.ctl.jump_to_app();
    }

    fn handle_idle(&mut self) -> ! {
        if self.platform.boot_state.boot_requested().unwrap_or(false) {
            log_info!("Boot requested, entering bootloader mode");
            self.platform.boot_state.transition().ok();
            self.enter_bootloader();
        }
        log_info!("Jumping to application");
        self.try_boot_app();
    }

    fn handle_updating(&mut self) -> ! {
        log_info!("Update in progress");
        self.enter_bootloader();
    }

    fn handle_validating(&mut self) -> ! {
        let remaining = self.platform.boot_state.trials_remaining().unwrap_or(0);
        if remaining == 0 {
            log_info!("Trial boots exhausted, entering bootloader mode");
            self.enter_bootloader();
        }
        log_info!("Trial boot ({} remaining)", remaining);
        self.platform.boot_state.increment_trial().ok();
        self.try_boot_app();
    }

    fn handle_confirmed(&mut self) -> ! {
        log_info!("Boot confirmed, resetting state");
        self.platform.boot_state.transition().ok(); // Confirmed → Idle (erase)
        self.try_boot_app();
    }

    fn enter_bootloader(&mut self) -> ! {
        log_info!("Entering bootloader mode");
        // TODO: firmware update loop over transport
        loop {}
    }
}
