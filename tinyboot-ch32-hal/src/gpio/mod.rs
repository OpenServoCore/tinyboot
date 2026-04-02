#[cfg_attr(gpio_v0, path = "v0.rs")]
#[cfg_attr(gpio_v3, path = "v3.rs")]
mod family;

pub use family::*;
