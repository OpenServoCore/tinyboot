use tinyboot::traits::BootState;
use tinyboot::traits::boot::BootMetaStore as TBBootMetaStore;
use tinyboot_ch32_hal::flash::FlashWriter;

const OB_BASE: u32 = 0x1FFFF800;
const META_OB_BASE: u32 = OB_BASE + 16;

#[derive(Debug)]
pub enum BootMetaError {
    InvalidTransition,
    TrialsExhausted,
}

/// CH32 boot metadata backed by option bytes.
///
/// Layout mirrors OB meta halfwords (data bytes at even addresses,
/// complement bytes skipped). Cached at construction, write-through on mutation.
#[repr(C)]
pub struct BootMetaStore {
    state: u8,
    trials: u8,
    checksum: u16,
    app_size: u32,
}

impl Default for BootMetaStore {
    /// Unlock flash and read all OB metadata from option bytes.
    #[inline(always)]
    fn default() -> Self {
        tinyboot_ch32_hal::flash::unlock();
        let mut meta = core::mem::MaybeUninit::<Self>::uninit();
        let ptr = meta.as_mut_ptr() as *mut u8;
        for i in 0..8 {
            unsafe {
                *ptr.add(i) = core::ptr::read_volatile((META_OB_BASE + i as u32 * 2) as *const u8);
            }
        }
        unsafe { meta.assume_init() }
    }
}

impl BootMetaStore {
    /// Erase OB and rewrite chip config + cached meta bytes.
    fn write(&self) {
        let mut buf = core::mem::MaybeUninit::<[u32; 4]>::uninit();
        let ptr = buf.as_mut_ptr() as *mut u8;
        // Read 8 chip config bytes from OB (stride-2 volatile reads)
        for i in 0..8 {
            unsafe {
                *ptr.add(i) = core::ptr::read_volatile((OB_BASE + i as u32 * 2) as *const u8);
            }
        }
        // Copy 8 meta struct bytes as two word copies
        let meta = self as *const Self as *const u32;
        let dst = unsafe { ptr.add(8) as *mut u32 };
        unsafe {
            *dst = *meta;
            *dst.add(1) = *meta.add(1);
        }
        let buf = unsafe { &*(buf.as_ptr() as *const [u8; 16]) };

        let w = FlashWriter::opt();
        w.erase_start();
        w.erase(OB_BASE);
        w.operation_end();
        w.write_start();
        let mut addr = OB_BASE;
        for &byte in buf.iter() {
            w.write(addr, byte as u16);
            addr += 2;
        }
        w.operation_end();
    }

    /// Bit-clear step down on a single OB byte. Updates cache + OB.
    fn step_down(&mut self, offset: usize, floor: u8) -> Option<u8> {
        let ptr = self as *mut Self as *mut u8;
        let current = unsafe { *ptr.add(offset) };
        if current <= floor {
            return None;
        }
        let next = current & (current >> 1);
        let w = FlashWriter::opt();
        w.write_start();
        w.write(META_OB_BASE + offset as u32 * 2, next as u16);
        w.operation_end();
        unsafe { *ptr.add(offset) = next };
        Some(next)
    }
}

impl TBBootMetaStore for BootMetaStore {
    type Error = BootMetaError;

    fn boot_state(&self) -> BootState {
        BootState::from_u8(self.state)
    }

    fn has_trials(&self) -> bool {
        self.trials != 0
    }

    fn app_checksum(&self) -> u16 {
        self.checksum
    }

    fn app_size(&self) -> u32 {
        self.app_size
    }

    fn advance(&mut self) -> Result<BootState, Self::Error> {
        let next = self
            .step_down(0, BootState::Validating as u8)
            .ok_or(BootMetaError::InvalidTransition)?;
        Ok(BootState::from_u8(next))
    }

    fn consume_trial(&mut self) -> Result<(), Self::Error> {
        self.step_down(1, 0).ok_or(BootMetaError::TrialsExhausted)?;
        Ok(())
    }

    fn refresh(
        &mut self,
        checksum: u16,
        state: BootState,
        app_size: u32,
    ) -> Result<(), Self::Error> {
        self.state = state as u8;
        self.trials = 0xFF;
        self.checksum = checksum;
        self.app_size = app_size;
        self.write();
        Ok(())
    }
}
