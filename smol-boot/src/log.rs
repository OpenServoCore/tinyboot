#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "defmt")]
        defmt::trace!($($arg)*)
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "defmt")]
        defmt::debug!($($arg)*)
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        #[cfg(feature = "defmt")]
        defmt::info!($($arg)*)
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "defmt")]
        defmt::warn!($($arg)*)
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        #[cfg(feature = "defmt")]
        defmt::error!($($arg)*)
    };
}
