#![allow(non_upper_case_globals)]

use std::ffi::CString;

extern crate win32_gamepads;
use win32_gamepads::*;

extern crate winapi;
use winapi::shared::minwindef::{DWORD, HMODULE};
use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryW};
use winapi::um::xinput::*;

#[macro_use]
extern crate log;

type XInputGetStateFunc = unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD;
type XInputSetStateFunc = unsafe extern "system" fn(DWORD, *mut XINPUT_VIBRATION) -> DWORD;

static mut global_xinput_handle: HMODULE = ::std::ptr::null_mut();
static mut opt_xinput_get_state: Option<XInputGetStateFunc> = None;
static mut opt_xinput_set_state: Option<XInputSetStateFunc> = None;

fn wide_null<S: AsRef<str>>(s: S) -> Vec<u16> {
  let mut output = vec![];
  for u in s.as_ref().encode_utf16() {
    output.push(u)
  }
  output.push(0);
  output
}

fn show_wide_null(arr: &[u16]) -> String {
  arr
    .iter()
    .take_while(|&&u| u != 0)
    .map(|&u| u as u8 as char)
    .collect()
}

unsafe fn dynamic_load_xinput() {
  let xinput14 = wide_null("xinput1_4.dll");
  let xinput91 = wide_null("xinput9_1_0.dll");
  let xinput13 = wide_null("xinput1_3.dll");

  let mut xinput_handle: HMODULE = ::std::ptr::null_mut();
  for lib_name in vec![xinput14, xinput91, xinput13] {
    trace!(
      "Attempting to load XInput DLL: {}",
      show_wide_null(&lib_name)
    );
    xinput_handle = LoadLibraryW(lib_name.as_ptr());
    if !xinput_handle.is_null() {
      debug!("Success: XInput Loaded: {}", show_wide_null(&lib_name));
      break;
    }
  }
  if xinput_handle.is_null() {
    debug!("Failure: XInput could not be loaded.");
  } else {
    let get_state_name = CString::new("XInputGetState").unwrap();
    let set_state_name = CString::new("XInputSetState").unwrap();

    let get_state_ptr = GetProcAddress(xinput_handle, get_state_name.as_ptr());
    if !get_state_ptr.is_null() {
      trace!("Found function {:?}.", get_state_name);
      opt_xinput_get_state = Some(::std::mem::transmute(get_state_ptr));
    } else {
      trace!("Could not find function {:?}.", get_state_name);
    }

    let set_state_ptr = GetProcAddress(xinput_handle, set_state_name.as_ptr());
    if !set_state_ptr.is_null() {
      trace!("Found Function {:?}.", set_state_name);
      opt_xinput_set_state = Some(::std::mem::transmute(set_state_ptr));
    } else {
      trace!("Could not find function {:?}.", set_state_name);
    }

    if opt_xinput_get_state.is_some() && opt_xinput_set_state.is_some() {
      global_xinput_handle = xinput_handle;
      debug!("Function pointers loaded successfully.");
    } else {
      opt_xinput_get_state = None;
      opt_xinput_set_state = None;
      FreeLibrary(xinput_handle);
      debug!("Could not load the function pointers.");
    }
  }
}

extern crate simple_logger;

fn main() {
  simple_logger::init().unwrap();
  unsafe {
    dynamic_load_xinput();

    gamepad();
  }
}
