extern crate win32_gamepads;
use win32_gamepads::*;

extern crate winapi;
use winapi::shared::minwindef::HMODULE;
use winapi::um::libloaderapi::LoadLibraryW;

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

unsafe fn do_xinput() {
  let xinput14 = wide_null("xinput1_4.dll");
  let xinput91 = wide_null("xinput9_1_0.dll");
  let xinput13 = wide_null("xinput1_3.dll");

  let mut loaded_xinput: HMODULE = ::std::ptr::null_mut();
  for lib_name in vec![xinput14, xinput91, xinput13] {
    loaded_xinput = LoadLibraryW(lib_name.as_ptr());
    if !loaded_xinput.is_null() {
      println!("Success: {}", show_wide_null(&lib_name));
      break;
    }
  }
  if loaded_xinput.is_null() {
    println!("Failure, could not load an XInput DLL.");
  }
}

fn main() {
  unsafe {
    do_xinput();

    gamepad();
  }
}
