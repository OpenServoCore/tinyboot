#![no_std]

#[cfg(not(feature = "ch32v003f4p6"))]
compile_error!(
    "No chip variant selected. Enable a chip feature, e.g.: \
     features = [\"ch32v003f4p6\"]"
);

#[cfg(not(any(feature = "bootloader", feature = "app")))]
compile_error!("Select either \"bootloader\" or \"app\" feature to indicate build role.");

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
pub use generated::{Pin, UsartMapping};

pub(crate) mod hal;

#[cfg(feature = "bootloader")]
pub mod boot;

#[cfg(feature = "app")]
pub mod app;
