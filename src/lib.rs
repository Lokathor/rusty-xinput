//! A library to allow easy access to all sorts of gamepads and game
//! controllers.

#![allow(non_upper_case_globals)]
#![warn(missing_docs)]
#![forbid(missing_debug_implementations)]

#[macro_use]
extern crate log;

#[cfg(target_os = "windows")]
extern crate winapi;

#[cfg(target_os = "windows")]
pub mod xinput;
#[cfg(target_os = "windows")]
pub use xinput::*;
