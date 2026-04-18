//! App hand-off strategies (one per build, gated on `system-flash`):
//! - `system`: software reset — ROM re-dispatches based on the latch.
//! - `user`: reset APB2 peripherals, then jump to the app reset vector.

core::cfg_select! {
    feature = "system-flash" => {
        mod system;
        pub type Active = system::SystemHandOff;
    }
    _ => {
        mod user;
        pub type Active = user::UserHandOff;
    }
}
