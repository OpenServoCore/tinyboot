use crate::protocol;
use crate::traits::{BootCtl, BootMetaStore, BootState, Platform, Storage, Transport};

pub struct Core<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    platform: Platform<T, S, B, C>,
}

impl<T, S, B, C> Core<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    pub fn new(platform: Platform<T, S, B, C>) -> Self {
        Core { platform }
    }

    pub fn run(mut self) -> ! {
        log_info!("Bootloader started");

        let mut enter = self.platform.ctl.is_boot_requested();

        if enter {
            log_info!("Boot requested");
            self.platform.boot_meta.advance().unwrap();
        } else {
            let meta = self.platform.boot_meta.read();
            match meta.boot_state() {
                BootState::Idle | BootState::Confirmed => {}
                BootState::Updating | BootState::Corrupt => enter = true,
                BootState::Validating => {
                    if meta.trials_remaining() == 0 {
                        enter = true;
                    } else {
                        self.platform.boot_meta.consume_trial().unwrap();
                    }
                }
            }
        }

        if enter || self.app_is_blank() {
            self.enter_bootloader();
        }
        self.platform.ctl.boot_app();
    }

    fn app_is_blank(&self) -> bool {
        let data = self.platform.storage.as_slice();
        data.len() < 4 || data[..4] == [0xFF; 4]
    }

    fn enter_bootloader(&mut self) -> ! {
        log_info!("Entering bootloader mode");

        let mut d = protocol::Dispatcher::new(&mut self.platform);

        loop {
            let _ = d.dispatch();
        }
    }
}
