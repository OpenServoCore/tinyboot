use crate::protocol;
use crate::traits::boot::{BootCtl, BootMetaStore, Platform, Storage, Transport};
use crate::traits::{BootMode, BootState};
use tinyboot_macros::tb_info;

/// Bootloader entry point. Checks boot state, validates the app, and either
/// boots the application or enters the protocol loop for firmware updates.
pub struct Core<T, S, B, C, const BUF: usize>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    platform: Platform<T, S, B, C>,
}

impl<T, S, B, C, const BUF: usize> Core<T, S, B, C, BUF>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    /// Create a new bootloader core from a platform implementation.
    #[inline(always)]
    pub fn new(platform: Platform<T, S, B, C>) -> Self {
        Core { platform }
    }

    /// Run the bootloader. Checks boot state, validates the app, and either
    /// boots the application or enters the protocol loop. Does not return.
    #[inline(always)]
    pub fn run(mut self) -> ! {
        tb_info!("Bootloader started");

        match self.check_boot_state() {
            Ok(BootMode::App) => self.platform.ctl.system_reset(BootMode::App),
            Ok(BootMode::Bootloader) | Err(_) => self.enter_bootloader(),
        }
    }

    fn check_boot_state(&mut self) -> Result<BootMode, B::Error> {
        if self.platform.ctl.is_boot_requested() {
            tb_info!("Boot requested");
            return Ok(BootMode::Bootloader);
        }

        match self.platform.boot_meta.boot_state() {
            BootState::Idle => {}
            BootState::Updating => return Ok(BootMode::Bootloader),
            BootState::Validating => {
                if !self.platform.boot_meta.has_trials() {
                    return Ok(BootMode::Bootloader);
                }
                self.platform.boot_meta.consume_trial()?;
            }
        }

        if !self.validate_app() {
            return Ok(BootMode::Bootloader);
        }

        Ok(BootMode::App)
    }

    /// Validate the app image. The CRC covers only `app_size` bytes
    /// (the actual firmware), not the entire flash region.
    fn validate_app(&self) -> bool {
        let stored = self.platform.boot_meta.app_checksum();
        if stored != 0xFFFF {
            use tinyboot_protocol::crc::{CRC_INIT, crc16};
            let sz = self.platform.boot_meta.app_size() as usize;
            // SAFETY: checksum != 0xFFFF means meta was previously written
            // by a Verify that validated app_size against capacity.
            return crc16(CRC_INIT, unsafe {
                self.platform.storage.as_slice().get_unchecked(..sz)
            }) == stored;
        }
        // No CRC stored (virgin/debugger-flashed) — check if app exists
        let data = self.platform.storage.as_slice();
        data.len() >= 4
            && unsafe { core::ptr::read_volatile(data.as_ptr() as *const u32) } != 0xFFFF_FFFF
    }

    #[inline(always)]
    fn enter_bootloader(&mut self) -> ! {
        tb_info!("Entering bootloader mode");
        self.platform.storage.unlock();

        let mut d = protocol::Dispatcher::<_, _, _, _, BUF>::new(&mut self.platform);

        loop {
            let _ = d.dispatch();
        }
    }
}
