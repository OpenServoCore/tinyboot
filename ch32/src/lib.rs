#![no_std]
#![allow(unexpected_cfgs)]

//! tinyboot bootloader and app library for CH32 microcontrollers.
//!
//! - [`hal`] — chip registers (flash, gpio, usart, …).
//! - [`platform`] — tinyboot-core trait impls.
//! - [`boot`] — bootloader entry point.
//! - [`app`] — app-side client.

#[cfg(not(any(
    feature = "ch32v002x4x6",
    feature = "ch32v003f4p6",
    feature = "ch32v003a4m6",
    feature = "ch32v003f4u6",
    feature = "ch32v003j4m6",
    feature = "ch32v004x6x1",
    feature = "ch32v005x6x6",
    feature = "ch32v006x8x6",
    feature = "ch32v007x8x6",
    feature = "ch32v103c6t6",
    feature = "ch32v103c8t6",
    feature = "ch32v103c8u6",
    feature = "ch32v103r8t6",
)))]
compile_error!(
    "No chip variant selected. Enable a chip feature, e.g.: \
     features = [\"ch32v003f4p6\"]"
);

pub mod app;
pub mod boot;
pub mod hal;
pub mod platform;

pub use ch32_metapac as pac;
