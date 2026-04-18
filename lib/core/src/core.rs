use crate::platform::Platform;
use crate::protocol;
use crate::traits::{BootCtl, BootMetaStore, BootState, RunMode, Storage, Transport};

/// Bootloader entry point. Validates the app and either hands off or enters
/// the protocol loop.
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
    /// Wrap a platform in a bootloader core.
    #[inline(always)]
    pub fn new(platform: Platform<T, S, B, C>) -> Self {
        Core { platform }
    }

    /// Run the bootloader. Does not return.
    #[inline(always)]
    pub fn run(mut self) -> ! {
        match self.check_boot_state() {
            Ok(RunMode::HandOff) => self.platform.ctl.hand_off(),
            Ok(RunMode::Service) | Err(_) => self.enter_bootloader(),
        }
    }

    fn check_boot_state(&mut self) -> Result<RunMode, B::Error> {
        if self.platform.ctl.run_mode() == RunMode::Service {
            return Ok(RunMode::Service);
        }

        match self.platform.boot_meta.boot_state() {
            BootState::Idle => {}
            BootState::Updating => return Ok(RunMode::Service),
            BootState::Validating => {
                if !self.platform.boot_meta.has_trials() {
                    return Ok(RunMode::Service);
                }
                self.platform.boot_meta.consume_trial()?;
            }
        }

        if !self.validate_app() {
            return Ok(RunMode::Service);
        }

        Ok(RunMode::HandOff)
    }

    /// Check the app image. CRC covers `app_size` bytes only, not the
    /// full region.
    fn validate_app(&self) -> bool {
        let stored = self.platform.boot_meta.app_checksum();
        if stored != 0xFFFF {
            use tinyboot_protocol::crc::{CRC_INIT, crc16};
            let sz = self.platform.boot_meta.app_size() as usize;
            // SAFETY: stored checksum implies Verify previously bounded app_size.
            return crc16(CRC_INIT, unsafe {
                self.platform.storage.as_slice().get_unchecked(..sz)
            }) == stored;
        }
        // No CRC stored (virgin / debugger-flashed) — require at least one
        // non-0xFF word to treat the app as present.
        let data = self.platform.storage.as_slice();
        data.len() >= 4
            && unsafe { core::ptr::read_volatile(data.as_ptr() as *const u32) } != 0xFFFF_FFFF
    }

    #[inline(always)]
    fn enter_bootloader(&mut self) -> ! {
        let mut d = protocol::Dispatcher::<_, _, _, _, BUF>::new(&mut self.platform);

        loop {
            let _ = d.dispatch();
        }
    }
}
