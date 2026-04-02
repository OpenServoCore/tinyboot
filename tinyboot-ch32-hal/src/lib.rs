#![no_std]
#![allow(unexpected_cfgs)]

#[cfg(not(any(
    feature = "ch32v003f4p6",
    feature = "ch32v003a4m6",
    feature = "ch32v003f4u6",
    feature = "ch32v003j4m6",
    feature = "ch32v103c6t6",
    feature = "ch32v103c8t6",
    feature = "ch32v103c8u6",
    feature = "ch32v103r8t6",
)))]
compile_error!(
    "No chip variant selected. Enable a chip feature, e.g.: \
     features = [\"ch32v003f4p6\"]"
);

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
pub use generated::{Pin, UsartMapping};

pub mod afio;
pub mod flash;
pub mod gpio;
pub mod iwdg;
pub mod pfic;
pub mod rcc;
pub mod usart;

pub mod boot_request;
