//! Boot-time platform container.

use crate::traits::{BootCtl, BootMetaStore, Storage, Transport};

/// Boot-time peripherals. Constructed by the board crate and passed to
/// [`Core::new`](crate::Core::new).
pub struct Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    /// UART / RS-485 transport.
    pub transport: T,
    /// App-region flash storage.
    pub storage: S,
    /// Persistent boot metadata.
    pub boot_meta: B,
    /// Boot control (reset, run mode, hand-off).
    pub ctl: C,
}

impl<T, S, B, C> Platform<T, S, B, C>
where
    T: Transport,
    S: Storage,
    B: BootMetaStore,
    C: BootCtl,
{
    /// Assemble a platform from its components.
    #[inline(always)]
    pub fn new(transport: T, storage: S, boot_meta: B, ctl: C) -> Self {
        Self {
            transport,
            storage,
            boot_meta,
            ctl,
        }
    }
}
