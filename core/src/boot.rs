use crate::hal::{APP_MAGIC, Abi, BootRequest, Registry};

#[derive(Debug, PartialEq)]
pub(crate) enum AppState {
    Valid,
    Invalid,
}

impl AppState {
    fn from_magic(magic: u32) -> Self {
        if magic == APP_MAGIC {
            AppState::Valid
        } else {
            AppState::Invalid
        }
    }
}

pub(crate) struct BootControl {
    app_state: AppState,
    boot_req: BootRequest,
}

impl BootControl {
    pub(crate) fn new(abi: &mut impl Abi, br: &mut impl Registry) -> Self {
        Self {
            app_state: AppState::from_magic(abi.app_magic()),
            boot_req: br.read_boot_request().unwrap_or(BootRequest::Invalid),
        }
    }

    pub(crate) fn should_boot_app(&self) -> bool {
        if self.app_state == AppState::Invalid {
            return false;
        }

        match self.boot_req {
            BootRequest::Invalid => true, // fresh chip, valid app, go for it
            BootRequest::Bootloader => false,
            BootRequest::Application => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::RegistryKey;

    // AppState::from_magic tests

    #[test]
    fn app_state_valid_magic() {
        assert_eq!(AppState::from_magic(APP_MAGIC), AppState::Valid);
    }

    #[test]
    fn app_state_wrong_magic() {
        assert_eq!(AppState::from_magic(0xDEADBEEF), AppState::Invalid);
    }

    #[test]
    fn app_state_erased_flash() {
        assert_eq!(AppState::from_magic(0xFFFFFFFF), AppState::Invalid);
    }

    #[test]
    fn app_state_zero() {
        assert_eq!(AppState::from_magic(0x00000000), AppState::Invalid);
    }

    // should_boot_app tests

    fn boot(app_state: AppState, boot_req: BootRequest) -> BootControl {
        BootControl {
            app_state,
            boot_req,
        }
    }

    #[test]
    fn no_magic_never_boots() {
        assert!(!boot(AppState::Invalid, BootRequest::Invalid).should_boot_app());
        assert!(!boot(AppState::Invalid, BootRequest::Application).should_boot_app());
        assert!(!boot(AppState::Invalid, BootRequest::Bootloader).should_boot_app());
    }

    #[test]
    fn valid_app_fresh_chip_boots() {
        assert!(boot(AppState::Valid, BootRequest::Invalid).should_boot_app());
    }

    #[test]
    fn valid_app_boot_request_application_boots() {
        assert!(boot(AppState::Valid, BootRequest::Application).should_boot_app());
    }

    #[test]
    fn valid_app_boot_request_bootloader_stays() {
        assert!(!boot(AppState::Valid, BootRequest::Bootloader).should_boot_app());
    }

    // BootControl::new tests

    struct MockABI {
        magic: u32,
        flash_region: (u32, u32),
    }

    impl Abi for MockABI {
        fn app_magic(&self) -> u32 {
            self.magic
        }

        fn app_flash_region(&self) -> (u32, u32) {
            self.flash_region
        }

        fn jump_to_app(&self) -> ! {
            loop {}
        }

        fn system_reset(&mut self) -> ! {
            loop {}
        }
    }

    struct MockRegistry {
        boot_req: Option<u8>,
    }

    impl Registry for MockRegistry {
        type Error = ();

        fn read(&mut self, key: RegistryKey) -> Result<u8, Self::Error> {
            match key {
                RegistryKey::BootRequest => self.boot_req.ok_or(()),
            }
        }
        fn write(&mut self, key: RegistryKey, value: u8) -> Result<(), Self::Error> {
            match key {
                RegistryKey::BootRequest => Ok(self.boot_req = Some(value)),
            }
        }
    }

    #[test]
    fn boot_control_valid_app_fresh_chip() {
        let mut abi = MockABI {
            magic: APP_MAGIC,
            flash_region: (0, 0),
        };
        let mut reg = MockRegistry {
            boot_req: Some(0x00),
        };
        assert!(BootControl::new(&mut abi, &mut reg).should_boot_app());
    }

    #[test]
    fn boot_control_valid_app_mode_application() {
        let mut abi = MockABI {
            magic: APP_MAGIC,
            flash_region: (0, 0),
        };
        let mut reg = MockRegistry {
            boot_req: Some(0x00),
        };
        assert!(BootControl::new(&mut abi, &mut reg).should_boot_app());
    }

    #[test]
    fn boot_control_valid_app_mode_bootloader() {
        let mut abi = MockABI {
            magic: APP_MAGIC,
            flash_region: (0, 0),
        };
        let mut reg = MockRegistry {
            boot_req: Some(0x01),
        };
        assert!(!BootControl::new(&mut abi, &mut reg).should_boot_app());
    }

    #[test]
    fn boot_control_no_app_fresh_chip() {
        let mut abi = MockABI {
            magic: 0xFFFFFFFF,
            flash_region: (0, 0),
        };
        let mut reg = MockRegistry { boot_req: None };
        assert!(!BootControl::new(&mut abi, &mut reg).should_boot_app());
    }

    #[test]
    fn boot_control_with_app_fresh_chip() {
        let mut abi = MockABI {
            magic: APP_MAGIC,
            flash_region: (0, 0),
        };
        let mut reg = MockRegistry { boot_req: None };
        assert!(BootControl::new(&mut abi, &mut reg).should_boot_app());
    }
}
