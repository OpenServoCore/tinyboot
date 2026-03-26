#![no_std]
#![warn(missing_docs)]

//! Platform-agnostic bootloader core.
//!
//! Implements the boot state machine, protocol dispatcher, and app validation.
//! Platform-specific behaviour is injected via the traits in [`traits::boot`].

/// App-side tinyboot client (poll, confirm, command handling).
pub mod app;
/// Boot state machine and entry point.
pub mod core;
/// Protocol frame dispatcher.
pub mod protocol;
/// Fixed-size ring buffer for buffered flash writes.
pub mod ringbuf;
/// Platform abstraction traits.
pub mod traits;

pub use crate::core::Core;
