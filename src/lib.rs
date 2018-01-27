//! A library to allow easy access to all sorts of gamepads and game
//! controllers.

#![no_std]
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

struct WideNullU16<'a>(&'a [u16; ::winapi::shared::minwindef::MAX_PATH]);

impl<'a> ::core::fmt::Debug for WideNullU16<'a> {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    for &u in self.0.iter() {
      if u == 0 {
        break;
      } else {
        write!(f, "{}", u as u8 as char)?
      }
    }
    Ok(())
  }
}

/// Converts a rusty string into a win32 string.
pub(crate) fn wide_null<S: AsRef<str>>(s: S) -> [u16; ::winapi::shared::minwindef::MAX_PATH] {
  let mut output: [u16; ::winapi::shared::minwindef::MAX_PATH] =
    [0; ::winapi::shared::minwindef::MAX_PATH];
  let mut i = 0;
  for u in s.as_ref().encode_utf16() {
    if i == output.len() - 1 {
      break;
    } else {
      output[i] = u;
    }
    i += 1;
  }
  output[i] = 0;
  output
}
