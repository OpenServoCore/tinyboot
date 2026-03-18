use crate::protocol;
use crate::traits::{BootCtl, BootMetaStore, Platform, Storage, Transport};

#[cfg(feature = "trial-boot")]
use crate::traits::BootState;

pub struct Core<const D: usize, T, S, B, C>
where
    T: Transport<D>,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    platform: Platform<D, T, S, B, C>,
}

impl<const D: usize, T, S, B, C> Core<D, T, S, B, C>
where
    T: Transport<D>,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    pub fn new(platform: Platform<D, T, S, B, C>) -> Self {
        Core { platform }
    }

    pub fn run(mut self) -> ! {
        log_info!("Bootloader started");

        match self.check_boot_state() {
            Ok(false) => self.platform.ctl.boot_app(),
            Ok(true) | Err(_) => self.enter_bootloader(),
        }
    }

    fn check_boot_state(&mut self) -> Result<bool, B::Error> {
        if self.platform.ctl.is_boot_requested() {
            log_info!("Boot requested");
            self.platform.boot_meta.advance()?;
            return Ok(true);
        }

        #[cfg(feature = "trial-boot")]
        {
            let meta = self.platform.boot_meta.read();
            match meta.boot_state() {
                BootState::Idle | BootState::Confirmed => {}
                BootState::Updating | BootState::Corrupt => return Ok(true),
                BootState::Validating => {
                    if meta.trials_remaining() == 0 {
                        return Ok(true);
                    }
                    self.platform.boot_meta.consume_trial()?;
                }
            }
        }

        if self.app_is_blank() {
            return Ok(true);
        }

        Ok(false)
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
