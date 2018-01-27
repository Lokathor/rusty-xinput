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

/// Converts a rusty string into a win32 string.
pub(crate) fn wide_null<S: AsRef<str>>(s: S) -> Vec<u16> {
  let mut output = vec![];
  for u in s.as_ref().encode_utf16() {
    output.push(u)
  }
  output.push(0);
  output
}

/// Converts a win32 string into a rusty string (ascii only).
pub(crate) fn show_wide_null(arr: &[u16]) -> String {
  arr
    .iter()
    .take_while(|&&u| u != 0)
    .map(|&u| u as u8 as char)
    .collect()
}
