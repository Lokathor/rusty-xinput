//! This module lets you load an XInput DLL and use it.
//!
//! ## How To Use This
//!
//! 1) Call `dynamic_load_xinput()`. This will attempt to load in a DLL that
//!    supports XInput. Note that the user might not have XInput installed, so
//!    be prepared to fall back to a keyboard/mouse if that happens.
//! 2) Call `xinput_get_state(controller)` to get your data. Usually you do this
//!    once at the start of each frame of the game. You can poll for controllers
//!    0, 1, 2, or 3. If a controller is connected you'll get `Ok(data)`.
//!    Otherwise you'll get some sort of `Err` info.
//! 3) Call `xinput_set_state(controller, left_speed, right_speed)` to set a
//!    rumble effect on the controller. As with `xinput_get_state`, you can
//!    select slots 0, 1, 2 or 3, and missing controllers or out of bounds
//!    selections will give an `Err` of some kind. Devices other than literal
//!    XBox 360 controllers have XInput drivers, but not all of them actually
//!    have rumble support, so this should be an extra not an essential.
//!
//! If xinput isn't fully loaded, a call to get_state or set_state is still
//! entirely safe to perform, you'll just get an `Err`.
//!
//! Note that there are theoretically other XInput extras you might care about,
//! but they're only available in Windows 8+ and I use Windows 7, so oh well.

#![allow(non_upper_case_globals)]
#![warn(missing_docs)]
#![forbid(missing_debug_implementations)]
#![cfg(windows)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

extern crate winapi;

use winapi::shared::guiddef::GUID;
use winapi::shared::minwindef::{BOOL, BYTE, DWORD, HMODULE, UINT, WORD};
use winapi::shared::ntdef::LPWSTR;
use winapi::shared::winerror::{ERROR_DEVICE_NOT_CONNECTED, ERROR_EMPTY, ERROR_SUCCESS};
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryW};
use winapi::um::xinput::*;

/// GetStateEx can get this in wButton
pub const XINPUT_GAMEPAD_GUIDE: winapi::shared::minwindef::WORD = 0x0400;

/// Capabilities info from the undocumented `XInputGetCapabilitiesEx` fn.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct XINPUT_CAPABILITIES_EX {
  capabilities: XINPUT_CAPABILITIES,
  vendor_id: WORD,
  product_id: WORD,
  revision_id: WORD,
  a4: DWORD,
}
impl ::std::fmt::Debug for XINPUT_CAPABILITIES_EX {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    write!(f, "XINPUT_CAPABILITIES_EX (_)")
  }
}

use std::fmt::{self, Debug, Formatter};

type XInputEnableFunc = unsafe extern "system" fn(BOOL);
type XInputGetStateFunc = unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD;
type XInputSetStateFunc = unsafe extern "system" fn(DWORD, *mut XINPUT_VIBRATION) -> DWORD;
type XInputGetCapabilitiesFunc =
  unsafe extern "system" fn(DWORD, DWORD, *mut XINPUT_CAPABILITIES) -> DWORD;

// undocumented
type XInputGetStateExFunc = unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD;

// undocumented
type XInputGetCapabilitiesEx =
  unsafe extern "system" fn(DWORD, DWORD, DWORD, *mut XINPUT_CAPABILITIES_EX) -> DWORD;

// **Removed** in xinput1_4.dll.
type XInputGetDSoundAudioDeviceGuidsFunc =
  unsafe extern "system" fn(DWORD, *mut GUID, *mut GUID) -> DWORD;

// Added in xinput1_3.dll.
type XInputGetKeystrokeFunc = unsafe extern "system" fn(DWORD, DWORD, PXINPUT_KEYSTROKE) -> DWORD;
type XInputGetBatteryInformationFunc =
  unsafe extern "system" fn(DWORD, BYTE, *mut XINPUT_BATTERY_INFORMATION) -> DWORD;

// Added in xinput1_4.dll.
type XInputGetAudioDeviceIdsFunc =
  unsafe extern "system" fn(DWORD, LPWSTR, *mut UINT, LPWSTR, *mut UINT) -> DWORD;

/// A handle to a loaded XInput DLL.
#[derive(Clone)]
pub struct XInputHandle {
  handle: HMODULE,
  xinput_enable: XInputEnableFunc,
  xinput_get_state: XInputGetStateFunc,
  xinput_set_state: XInputSetStateFunc,
  xinput_get_capabilities: XInputGetCapabilitiesFunc,
  opt_xinput_get_state_ex: Option<XInputGetStateExFunc>,
  opt_xinput_get_capabilities_ex: Option<XInputGetCapabilitiesEx>,
  opt_xinput_get_keystroke: Option<XInputGetKeystrokeFunc>,
  opt_xinput_get_battery_information: Option<XInputGetBatteryInformationFunc>,
  // some day we should use these
  _opt_xinput_get_audio_device_ids: Option<XInputGetAudioDeviceIdsFunc>,
  _opt_xinput_get_dsound_audio_device_guids: Option<XInputGetDSoundAudioDeviceGuidsFunc>,
}

impl Debug for XInputHandle {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    write!(f, "XInputHandle(handle = {:?})", self.handle)
  }
}

unsafe impl Send for XInputHandle {}
unsafe impl Sync for XInputHandle {}

lazy_static! {
  static ref GLOBAL_XINPUT_HANDLE: Result<XInputHandle, XInputLoadingFailure> =
    XInputHandle::load_default();
}

/// Quick and dirty wrapper to let us format log messages easier.
pub(crate) struct WideNullU16<'a>(&'a [u16; ::winapi::shared::minwindef::MAX_PATH]);
impl<'a> ::std::fmt::Debug for WideNullU16<'a> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
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

/// The ways that a dynamic load of XInput can fail.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XInputLoadingFailure {
  /// The xinput system was already in the process of loading in some other
  /// thread. This attempt failed because of that, but that other attempt might
  /// still succeed.
  #[deprecated]
  AlreadyLoading,
  /// The xinput system was already active. A failure of this kind leaves the
  /// system active.
  AlreadyActive,
  /// The system was not loading or active, but was in some unknown state. If
  /// you get this, it's probably a bug that you should report.
  UnknownState,
  /// No DLL for XInput could be found. This places the system back into an
  /// "uninitialized" status, and you could potentially try again later if the
  /// user fiddles with the program's DLL path or whatever.
  NoDLL,
  /// A DLL was found that matches one of the expected XInput DLL names, but it
  /// didn't contain both of the expected functions. This is probably a weird
  /// situation to find. Either way, the xinput status is set to "uninitialized"
  /// and as with the NoDLL error you could potentially try again.
  NoPointers,
}

impl XInputHandle {
  /// Attempts to dynamically load an XInput DLL and get the function pointers.
  ///
  /// # Failure
  ///
  /// This can fail in a few ways, as explained in the `XInputLoadingFailure`
  /// type. The most likely failure case is that the user's system won't have the
  /// required DLL, in which case you should probably allow them to play with just
  /// a keyboard/mouse instead.
  ///
  /// # Current DLL Names
  ///
  /// Currently the following DLL names are searched for in this order:
  ///
  /// * `xinput1_4.dll`
  /// * `xinput1_3.dll`
  /// * `xinput1_2.dll`
  /// * `xinput1_1.dll`
  /// * `xinput9_1_0.dll`
  pub fn load_default() -> Result<XInputHandle, XInputLoadingFailure> {
    let xinput14 = "xinput1_4.dll";
    let xinput13 = "xinput1_3.dll";
    let xinput12 = "xinput1_2.dll";
    let xinput11 = "xinput1_1.dll";
    let xinput91 = "xinput9_1_0.dll";

    for lib_name in [xinput14, xinput13, xinput12, xinput11, xinput91] {
      if let Ok(handle) = XInputHandle::load(lib_name) {
        return Ok(handle);
      }
    }

    debug!("Failure: XInput could not be loaded.");
    Err(XInputLoadingFailure::NoDLL)
  }

  /// Attempt to load a specific XInput DLL and get the function pointers.
  pub fn load<S: AsRef<str>>(s: S) -> Result<XInputHandle, XInputLoadingFailure> {
    let lib_name = wide_null(s);
    trace!(
      "Attempting to load XInput DLL: {:?}",
      WideNullU16(&lib_name)
    );
    // It's always safe to call `LoadLibraryW`, the worst that can happen is
    // that we get a null pointer back.
    let xinput_handle = unsafe { LoadLibraryW(lib_name.as_ptr()) };
    if !xinput_handle.is_null() {
      debug!("Success: XInput Loaded: {:?}", WideNullU16(&lib_name));
    }

    let enable_name = b"XInputEnable\0";
    let get_state_name = b"XInputGetState\0";
    let set_state_name = b"XInputSetState\0";
    let get_capabilities_name = b"XInputGetCapabilities\0";
    let get_keystroke_name = b"XInputGetKeystroke\0";
    let get_battery_information_name = b"XInputGetBatteryInformation\0";
    let get_audio_device_ids_name = b"XInputGetAudioDeviceIds\0";
    let get_dsound_audio_device_guids_name = b"XInputGetDSoundAudioDeviceGuids\0";

    let mut opt_xinput_enable = None;
    let mut opt_xinput_get_state = None;
    let mut opt_xinput_get_state_ex = None;
    let mut opt_xinput_set_state = None;
    let mut opt_xinput_get_capabilities = None;
    let mut opt_xinput_get_capabilities_ex = None;
    let mut opt_xinput_get_keystroke = None;
    let mut opt_xinput_get_battery_information = None;
    let mut opt_xinput_get_audio_device_ids = None;
    let mut opt_xinput_get_dsound_audio_device_guids = None;

    unsafe {
      let enable_ptr = GetProcAddress(xinput_handle, enable_name.as_ptr() as *mut i8);
      if !enable_ptr.is_null() {
        trace!("Found XInputEnable.");
        opt_xinput_enable = Some(::std::mem::transmute(enable_ptr));
      } else {
        trace!("Could not find XInputEnable.");
      }
    }

    unsafe {
      let get_state_ptr = GetProcAddress(xinput_handle, get_state_name.as_ptr() as *mut i8);
      if !get_state_ptr.is_null() {
        trace!("Found XInputGetState.");
        opt_xinput_get_state = Some(::std::mem::transmute(get_state_ptr));
      } else {
        trace!("Could not find XInputGetState.");
      }
    }

    unsafe {
      let set_state_ptr = GetProcAddress(xinput_handle, set_state_name.as_ptr() as *mut i8);
      if !set_state_ptr.is_null() {
        trace!("Found XInputSetState.");
        opt_xinput_set_state = Some(::std::mem::transmute(set_state_ptr));
      } else {
        trace!("Could not find XInputSetState.");
      }
    }

    unsafe {
      let get_state_ex_ptr = GetProcAddress(xinput_handle, 100_i32 as winapi::um::winnt::LPCSTR);
      if !get_state_ex_ptr.is_null() {
        trace!("Found XInputGetStateEx.");
        opt_xinput_get_state_ex = Some(::std::mem::transmute(get_state_ex_ptr));
      } else {
        trace!("Could not find XInputGetStateEx.");
      }
    }

    unsafe {
      let get_capabilities_ptr =
        GetProcAddress(xinput_handle, get_capabilities_name.as_ptr() as *mut i8);
      if !get_capabilities_ptr.is_null() {
        trace!("Found XInputGetCapabilities.");
        opt_xinput_get_capabilities = Some(::std::mem::transmute(get_capabilities_ptr));
      } else {
        trace!("Could not find XInputGetCapabilities.");
      }
    }

    unsafe {
      let get_capabilities_ptr =
        GetProcAddress(xinput_handle, 108_i32 as winapi::um::winnt::LPCSTR);
      if !get_capabilities_ptr.is_null() {
        trace!("Found XInputGetCapabilities.");
        opt_xinput_get_capabilities_ex = Some(::std::mem::transmute(get_capabilities_ptr));
      } else {
        trace!("Could not find XInputGetCapabilitiesEx.");
      }
    }

    unsafe {
      let get_keystroke_ptr = GetProcAddress(xinput_handle, get_keystroke_name.as_ptr() as *mut i8);
      if !get_keystroke_ptr.is_null() {
        trace!("Found XInputGetKeystroke.");
        opt_xinput_get_keystroke = Some(::std::mem::transmute(get_keystroke_ptr));
      } else {
        trace!("Could not find XInputGetKeystroke.");
      }
    }

    unsafe {
      let get_battery_information_ptr = GetProcAddress(
        xinput_handle,
        get_battery_information_name.as_ptr() as *mut i8,
      );
      if !get_battery_information_ptr.is_null() {
        trace!("Found XInputGetBatteryInformation.");
        opt_xinput_get_battery_information =
          Some(::std::mem::transmute(get_battery_information_ptr));
      } else {
        trace!("Could not find XInputGetBatteryInformation.");
      }
    }

    unsafe {
      let get_dsound_audio_device_guids_ptr = GetProcAddress(
        xinput_handle,
        get_dsound_audio_device_guids_name.as_ptr() as *mut i8,
      );
      if !get_dsound_audio_device_guids_ptr.is_null() {
        trace!("Found XInputGetDSoundAudioDeviceGuids.");
        opt_xinput_get_dsound_audio_device_guids =
          Some(::std::mem::transmute(get_dsound_audio_device_guids_ptr));
      } else {
        trace!("Could not find XInputGetDSoundAudioDeviceGuids.");
      }
    }

    unsafe {
      let get_audio_device_ids_ptr =
        GetProcAddress(xinput_handle, get_audio_device_ids_name.as_ptr() as *mut i8);
      if !get_audio_device_ids_ptr.is_null() {
        trace!("Found XInputGetAudioDeviceIds.");
        opt_xinput_get_audio_device_ids = Some(::std::mem::transmute(get_audio_device_ids_ptr));
      } else {
        trace!("Could not find XInputGetAudioDeviceIds.");
      }
    }

    #[allow(clippy::unnecessary_unwrap)]
    if opt_xinput_enable.is_some()
      && opt_xinput_get_state.is_some()
      && opt_xinput_set_state.is_some()
      && opt_xinput_get_capabilities.is_some()
    {
      debug!("All function pointers loaded successfully.");
      Ok(XInputHandle {
        handle: xinput_handle,
        xinput_enable: opt_xinput_enable.unwrap(),
        xinput_get_state: opt_xinput_get_state.unwrap(),
        xinput_set_state: opt_xinput_set_state.unwrap(),
        xinput_get_capabilities: opt_xinput_get_capabilities.unwrap(),
        opt_xinput_get_capabilities_ex,
        opt_xinput_get_state_ex,
        opt_xinput_get_keystroke,
        opt_xinput_get_battery_information,
        _opt_xinput_get_dsound_audio_device_guids: opt_xinput_get_dsound_audio_device_guids,
        _opt_xinput_get_audio_device_ids: opt_xinput_get_audio_device_ids,
      })
    } else {
      debug!("Could not load the function pointers.");
      Err(XInputLoadingFailure::NoPointers)
    }
  }
}

/// Attempts to dynamically load an XInput DLL and get the function pointers.
///
/// This operation is thread-safe and can be performed at any time. If xinput
/// hasn't been loaded yet, or if there was a failed load attempt, then
/// `xinput_get_state` and `xinput_set_state` will safety return an `Err` value
/// to that effect.
///
/// There's no way provided to unload XInput once it's been loaded, because that
/// makes the normal operation a little faster. Why would you want to unload it
/// anyway? Don't be silly.
///
/// # Failure
///
/// This can fail in a few ways, as explained in the `XInputLoadingFailure`
/// type. The most likely failure case is that the user's system won't have the
/// required DLL, in which case you should probably allow them to play with just
/// a keyboard/mouse instead.
///
/// # Current DLL Names
///
/// Currently the following DLL names are searched for in this order:
///
/// * `xinput9_1_0.dll`
/// * `xinput1_4.dll`
/// * `xinput1_3.dll`
/// * `xinput1_2.dll`
/// * `xinput1_1.dll`
#[deprecated]
pub fn dynamic_load_xinput() -> Result<(), XInputLoadingFailure> {
  if let Err(err) = *GLOBAL_XINPUT_HANDLE {
    Err(err)
  } else {
    Ok(())
  }
}

/// This wraps an `XINPUT_STATE` value and provides a more rusty (read-only)
/// interface to the data it contains.
///
/// All three major game companies use different names for most of the buttons,
/// so the docs for each button method list out what each of the major companies
/// call that button. To the driver it's all the same, it's just however you
/// want to think of them.
///
/// If sequential calls to `xinput_get_state` for a given controller slot have
/// the same packet number then the controller state has not changed since the
/// last call. The `PartialEq` and `Eq` implementations for this wrapper type
/// reflect that. The exact value of the packet number is unimportant.
///
/// If you want to do something that the rust wrapper doesn't support, just use
/// the raw field to get at the inner value.
#[derive(Copy, Clone)]
pub struct XInputState {
  /// The raw value we're wrapping.
  pub raw: XINPUT_STATE,
}

impl ::std::default::Default for XInputState {
  #[inline]
  #[must_use]
  fn default() -> Self {
    Self {
      raw: XINPUT_STATE {
        dwPacketNumber: 0,
        Gamepad: XINPUT_GAMEPAD {
          wButtons: 0,
          bLeftTrigger: 0,
          bRightTrigger: 0,
          sThumbLX: 0,
          sThumbLY: 0,
          sThumbRX: 0,
          sThumbRY: 0,
        },
      },
    }
  }
}

impl ::std::cmp::PartialEq for XInputState {
  /// Equality for `XInputState` values is based _only_ on the
  /// `dwPacketNumber` of the wrapped `XINPUT_STATE` value. This is entirely
  /// correct for values obtained from the xinput system, but if you make your
  /// own `XInputState` values for some reason you can confuse it.
  fn eq(&self, other: &XInputState) -> bool {
    self.raw.dwPacketNumber == other.raw.dwPacketNumber
  }
}

impl ::std::cmp::Eq for XInputState {}

impl ::std::fmt::Debug for XInputState {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    write!(f, "XInputState (_)")
  }
}

impl XInputState {
  /// The north button of the action button group.
  ///
  /// * Nintendo: X
  /// * Playstation: Triangle
  /// * XBox: Y
  #[inline]
  pub fn north_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_Y != 0
  }

  /// The south button of the action button group.
  ///
  /// * Nintendo: B
  /// * Playstation: X
  /// * XBox: A
  #[inline]
  pub fn south_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_A != 0
  }

  /// The east button of the action button group.
  ///
  /// * Nintendo: A
  /// * Playstation: Circle
  /// * XBox: B
  #[inline]
  pub fn east_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_B != 0
  }

  /// The west button of the action button group.
  ///
  /// * Nintendo: Y
  /// * Playstation: Square
  /// * XBox: X
  #[inline]
  pub fn west_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_X != 0
  }

  /// The up button on the directional pad.
  #[inline]
  pub fn arrow_up(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_DPAD_UP != 0
  }

  /// The down button on the directional pad.
  #[inline]
  pub fn arrow_down(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0
  }

  /// The left button on the directional pad.
  #[inline]
  pub fn arrow_left(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0
  }

  /// The right button on the directional pad.
  #[inline]
  pub fn arrow_right(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0
  }

  /// The "start" button.
  ///
  /// * Nintendo: Start (NES / SNES), '+' (Pro Controller)
  /// * Playstation: Start
  /// * XBox: Start
  #[inline]
  pub fn start_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_START != 0
  }

  /// The "not start" button.
  ///
  /// * Nintendo: Select (NES / NES), '-' (Pro Controller)
  /// * Playstation: Select
  /// * XBox: Back
  #[inline]
  pub fn select_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_BACK != 0
  }

  /// The "guide" button.
  ///
  /// * Nintendo: Home
  /// * Playstation: PS
  /// * XBox: Guide
  #[inline]
  pub fn guide_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_GUIDE != 0
  }

  /// The upper left shoulder button.
  ///
  /// * Nintendo: L
  /// * Playstation: L1
  /// * XBox: LB
  #[inline]
  pub fn left_shoulder(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER != 0
  }

  /// The upper right shoulder button.
  ///
  /// * Nintendo: R
  /// * Playstation: R1
  /// * XBox: RB
  #[inline]
  pub fn right_shoulder(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER != 0
  }

  /// The default threshold to count a trigger as being "pressed".
  pub const TRIGGER_THRESHOLD: u8 = XINPUT_GAMEPAD_TRIGGER_THRESHOLD;

  /// The lower left shoulder trigger. If you want to use this as a simple
  /// boolean it is suggested that you compare it to the `TRIGGER_THRESHOLD`
  /// constant.
  ///
  /// * Nintendo: ZL
  /// * Playstation: L2
  /// * XBox: LT
  #[inline]
  pub fn left_trigger(&self) -> u8 {
    self.raw.Gamepad.bLeftTrigger
  }

  /// The lower right shoulder trigger. If you want to use this as a simple
  /// boolean it is suggested that you compare it to the `TRIGGER_THRESHOLD`
  /// constant.
  ///
  /// * Nintendo: ZR
  /// * Playstation: R2
  /// * XBox: RT
  #[inline]
  pub fn right_trigger(&self) -> u8 {
    self.raw.Gamepad.bRightTrigger
  }

  /// The lower left shoulder trigger as a bool using the default threshold.
  ///
  /// * Nintendo: ZL
  /// * Playstation: L2
  /// * XBox: LT
  #[inline]
  pub fn left_trigger_bool(&self) -> bool {
    self.left_trigger() >= XInputState::TRIGGER_THRESHOLD
  }

  /// The lower right shoulder trigger as a bool using the default threshold.
  ///
  /// * Nintendo: ZR
  /// * Playstation: R2
  /// * XBox: RT
  #[inline]
  pub fn right_trigger_bool(&self) -> bool {
    self.right_trigger() >= XInputState::TRIGGER_THRESHOLD
  }

  /// The left thumb stick being pressed inward.
  ///
  /// * Nintendo: (L)
  /// * Playstation: L3
  /// * XBox: (L)
  #[inline]
  pub fn left_thumb_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_LEFT_THUMB != 0
  }

  /// The right thumb stick being pressed inward.
  ///
  /// * Nintendo: (R)
  /// * Playstation: R3
  /// * XBox: (R)
  #[inline]
  pub fn right_thumb_button(&self) -> bool {
    self.raw.Gamepad.wButtons & XINPUT_GAMEPAD_RIGHT_THUMB != 0
  }

  /// The suggested default deadzone for use with the left thumb stick.
  pub const LEFT_STICK_DEADZONE: i16 = XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE;

  /// The suggested default deadzone for use with the right thumb stick.
  pub const RIGHT_STICK_DEADZONE: i16 = XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE;

  /// The left stick raw value.
  ///
  /// Positive values are to the right (X-axis) or up (Y-axis).
  #[inline]
  pub fn left_stick_raw(&self) -> (i16, i16) {
    (self.raw.Gamepad.sThumbLX, self.raw.Gamepad.sThumbLY)
  }

  /// The right stick raw value.
  ///
  /// Positive values are to the right (X-axis) or up (Y-axis).
  #[inline]
  pub fn right_stick_raw(&self) -> (i16, i16) {
    (self.raw.Gamepad.sThumbRX, self.raw.Gamepad.sThumbRY)
  }

  /// The left stick value normalized with the default dead-zone.
  ///
  /// See `normalize_raw_stick_value` for more.
  #[inline]
  pub fn left_stick_normalized(&self) -> (f32, f32) {
    XInputState::normalize_raw_stick_value(self.left_stick_raw(), XInputState::LEFT_STICK_DEADZONE)
  }

  /// The right stick value normalized with the default dead-zone.
  ///
  /// See `normalize_raw_stick_value` for more.
  #[inline]
  pub fn right_stick_normalized(&self) -> (f32, f32) {
    XInputState::normalize_raw_stick_value(
      self.right_stick_raw(),
      XInputState::RIGHT_STICK_DEADZONE,
    )
  }

  /// This helper normalizes a raw stick value using the given deadzone.
  ///
  /// If the raw value's 2d length is less than the deadzone the result will be
  /// `(0.0,0.0)`, otherwise the result is normalized across the range from the
  /// deadzone point to the maximum value.
  ///
  /// The `deadzone` value is clamped to the range 0 to 32,766 (inclusive)
  /// before use. Negative inputs or maximum value inputs make the normalization
  /// just work improperly.
  #[inline]
  pub fn normalize_raw_stick_value(raw_stick: (i16, i16), deadzone: i16) -> (f32, f32) {
    let deadzone_float = deadzone.max(0).min(i16::max_value() - 1) as f32;
    let raw_float = (raw_stick.0 as f32, raw_stick.1 as f32);
    let length = (raw_float.0 * raw_float.0 + raw_float.1 * raw_float.1).sqrt();
    let normalized = (raw_float.0 / length, raw_float.1 / length);
    if length > deadzone_float {
      // clip our value to the expected maximum length.
      let length = length.min(32_767.0);
      let scale = (length - deadzone_float) / (32_767.0 - deadzone_float);
      (normalized.0 * scale, normalized.1 * scale)
    } else {
      (0.0, 0.0)
    }
  }
}

#[test]
#[rustfmt::skip]
fn normalize_raw_stick_value_test() {
  for x in [i16::min_value(), i16::max_value()] {
    for y in [i16::min_value(), i16::max_value()] {
      for deadzone in [i16::min_value(), 0, i16::max_value() / 2,
                        i16::max_value() - 1, i16::max_value()] {
        let f = XInputState::normalize_raw_stick_value((x, y), deadzone);
        assert!(f.0.abs() <= 1.0, "XFail: x {}, y {}, dz {} f {:?}", x, y, deadzone, f);
        assert!(f.1.abs() <= 1.0, "YFail: x {}, y {}, dz {} f {:?}", x, y, deadzone, f);
      }
    }
  }
}

/// These are all the sorts of problems that can come up when you're using the
/// xinput system.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XInputUsageError {
  /// XInput isn't currently loaded.
  XInputNotLoaded,
  /// The controller ID you gave was 4 or more.
  InvalidControllerID,
  /// Not really an error, this controller is just missing.
  DeviceNotConnected,
  /// There was some sort of unexpected error happened, this is the error code
  /// windows returned.
  UnknownError(u32),
}

/// Error that can be returned by functions that are not guaranteed to be present
/// in earlier XInput versions.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XInputOptionalFnUsageError {
  /// XInput isn't currently loaded.
  XInputNotLoaded,
  /// The controller ID you gave was 4 or more.
  InvalidControllerID,
  /// Not really an error, this controller is just missing.
  DeviceNotConnected,
  /// Function is not present in loaded DLL
  FunctionNotLoaded,
  /// There was some sort of unexpected error happened, this is the error code
  /// windows returned.
  UnknownError(u32),
}

impl XInputHandle {
  /// Enables or disables XInput.
  ///
  /// See the [MSDN documentation for XInputEnable](https://docs.microsoft.com/en-us/windows/desktop/api/xinput/nf-xinput-xinputenable).
  pub fn enable(&self, enable: bool) {
    unsafe { (self.xinput_enable)(enable as BOOL) };
  }

  /// Polls the controller port given for the current controller state.
  ///
  /// This cannot detect the "Guide" button. Use
  /// [`get_state_ex`](Self::get_state_ex) for that.
  ///
  /// # Notes
  ///
  /// It is a persistent problem with xinput (since ~2007?) that polling for the
  /// data of a controller that isn't connected will cause a long stall. In the
  /// area of 500,000 cpu cycles. That's like 2,000 cache misses in a row.
  ///
  /// Once a controller is detected as not being plugged in you are strongly
  /// advised to not poll for its data again next frame. Instead, you should
  /// probably only poll for one known-missing controller per frame at most.
  ///
  /// Alternately, you can register for your app to get plug and play events and
  /// then wait for one of them to come in before you ever poll for a missing
  /// controller a second time. That's up to you.
  ///
  /// # Errors
  ///
  /// A few things can cause an `Err` value to come back, as explained by the
  /// `XInputUsageError` type.
  ///
  /// Most commonly, a controller will simply not be connected. Most people
  /// don't have all four slots plugged in all the time.
  pub fn get_state(&self, user_index: u32) -> Result<XInputState, XInputUsageError> {
    if user_index >= 4 {
      Err(XInputUsageError::InvalidControllerID)
    } else {
      let mut output: XINPUT_STATE = unsafe { ::std::mem::zeroed() };
      let return_status = unsafe { (self.xinput_get_state)(user_index, &mut output) };
      match return_status {
        ERROR_SUCCESS => Ok(XInputState { raw: output }),
        ERROR_DEVICE_NOT_CONNECTED => Err(XInputUsageError::DeviceNotConnected),
        s => {
          trace!("Unexpected error code: {}", s);
          Err(XInputUsageError::UnknownError(s))
        }
      }
    }
  }

  /// Works like `get_state`, but can detect the "Guide" button as well.
  ///
  /// ## Failure
  ///
  /// * This function is technically an undocumented API. It was introduced in
  ///   XInput 1.3, but may not be present in the currently loaded XInput. If
  ///   it's not available then `XInputNotLoaded` is returned as an `Err`, even
  ///   when other XInput functions may be available.
  pub fn get_state_ex(&self, user_index: u32) -> Result<XInputState, XInputUsageError> {
    if user_index >= 4 {
      Err(XInputUsageError::InvalidControllerID)
    } else {
      let mut output: XINPUT_STATE = unsafe { ::std::mem::zeroed() };
      let return_status = match self.opt_xinput_get_state_ex {
        Some(f) => unsafe { f(user_index, &mut output) },
        None => return Err(XInputUsageError::XInputNotLoaded),
      };
      match return_status {
        ERROR_SUCCESS => Ok(XInputState { raw: output }),
        ERROR_DEVICE_NOT_CONNECTED => Err(XInputUsageError::DeviceNotConnected),
        s => {
          trace!("Unexpected error code: {}", s);
          Err(XInputUsageError::UnknownError(s))
        }
      }
    }
  }
}

/// See `XInputHandle::get_state`
#[deprecated]
pub fn xinput_get_state(user_index: u32) -> Result<XInputState, XInputUsageError> {
  match *GLOBAL_XINPUT_HANDLE {
    Ok(ref handle) => handle.get_state(user_index),
    Err(_) => Err(XInputUsageError::XInputNotLoaded),
  }
}

impl XInputHandle {
  /// Allows you to set the rumble speeds of the left and right motors.
  ///
  /// Valid motor speeds are across the whole `u16` range, and the number is the
  /// scale of the motor intensity. In other words, 0 is 0%, and 65,535 is 100%.
  ///
  /// On a 360 controller the left motor is low-frequency and the right motor is
  /// high-frequency. On other controllers running through xinput this might be
  /// the case, or the controller might not even have rumble ability at all. If
  /// rumble is missing from the device you'll still get `Ok` return values, so
  /// treat rumble as an extra, not an essential.
  ///
  /// # Errors
  ///
  /// A few things can cause an `Err` value to come back, as explained by the
  /// `XInputUsageError` type.
  ///
  /// Most commonly, a controller will simply not be connected. Most people don't
  /// have all four slots plugged in all the time.
  pub fn set_state(
    &self,
    user_index: u32,
    left_motor_speed: u16,
    right_motor_speed: u16,
  ) -> Result<(), XInputUsageError> {
    if user_index >= 4 {
      Err(XInputUsageError::InvalidControllerID)
    } else {
      let mut input = XINPUT_VIBRATION {
        wLeftMotorSpeed: left_motor_speed,
        wRightMotorSpeed: right_motor_speed,
      };
      let return_status = unsafe { (self.xinput_set_state)(user_index, &mut input) };
      match return_status {
        ERROR_SUCCESS => Ok(()),
        ERROR_DEVICE_NOT_CONNECTED => Err(XInputUsageError::DeviceNotConnected),
        s => {
          trace!("Unexpected error code: {}", s);
          Err(XInputUsageError::UnknownError(s))
        }
      }
    }
  }
}

/// See `XInputHandle::set_state`
#[deprecated]
pub fn xinput_set_state(
  user_index: u32,
  left_motor_speed: u16,
  right_motor_speed: u16,
) -> Result<(), XInputUsageError> {
  match *GLOBAL_XINPUT_HANDLE {
    Ok(ref handle) => handle.set_state(user_index, left_motor_speed, right_motor_speed),
    Err(_) => Err(XInputUsageError::XInputNotLoaded),
  }
}

impl XInputHandle {
  /// Retrieve the capabilities of a controller.
  ///
  /// See the [MSDN documentation for XInputGetCapabilities](https://docs.microsoft.com/en-us/windows/desktop/api/xinput/nf-xinput-xinputgetcapabilities).
  pub fn get_capabilities(&self, user_index: u32) -> Result<XINPUT_CAPABILITIES, XInputUsageError> {
    if user_index >= 4 {
      Err(XInputUsageError::InvalidControllerID)
    } else {
      unsafe {
        let mut capabilities = std::mem::zeroed();
        let return_status = (self.xinput_get_capabilities)(user_index, 0, &mut capabilities);
        match return_status {
          ERROR_SUCCESS => Ok(capabilities),
          ERROR_DEVICE_NOT_CONNECTED => Err(XInputUsageError::DeviceNotConnected),
          s => {
            trace!("Unexpected error code: {}", s);
            Err(XInputUsageError::UnknownError(s))
          }
        }
      }
    }
  }
  /// Retrieve the Extended capabilities of a controller.
  ///
  /// Undocumented!! This isn't part of the official XInput API, but is often available.
  ///
  /// ## Failure
  ///
  /// * This function is technically an undocumented API. If
  ///   it's not available then `XInputNotLoaded` is returned as an `Err`, even
  ///   when other XInput functions may be available.
  pub fn get_capabilities_ex(
    &self,
    user_index: u32,
  ) -> Result<XINPUT_CAPABILITIES_EX, XInputUsageError> {
    if user_index >= 4 {
      Err(XInputUsageError::InvalidControllerID)
    } else {
      unsafe {
        let mut capabilities_ex = std::mem::zeroed();
        let return_status = match self.opt_xinput_get_capabilities_ex {
          None => return Err(XInputUsageError::XInputNotLoaded),
          Some(f) => f(1, user_index, 0, &mut capabilities_ex),
        };
        match return_status {
          ERROR_SUCCESS => Ok(capabilities_ex),
          ERROR_DEVICE_NOT_CONNECTED => Err(XInputUsageError::DeviceNotConnected),
          s => {
            trace!("Unexpected error code: {}", s);
            Err(XInputUsageError::UnknownError(s))
          }
        }
      }
    }
  }

  /// Retrieve a gamepad input event.
  ///
  /// See the [MSDN documentation for XInputGetKeystroke](https://docs.microsoft.com/en-us/windows/desktop/api/xinput/nf-xinput-xinputgetkeystroke).
  pub fn get_keystroke(
    &self,
    user_index: u32,
  ) -> Result<Option<XINPUT_KEYSTROKE>, XInputOptionalFnUsageError> {
    if user_index >= 4 {
      Err(XInputOptionalFnUsageError::InvalidControllerID)
    } else if let Some(func) = self.opt_xinput_get_keystroke {
      unsafe {
        let mut keystroke = std::mem::zeroed();
        let return_status = (func)(user_index, 0, &mut keystroke);
        match return_status {
          ERROR_SUCCESS => Ok(Some(keystroke)),
          ERROR_EMPTY => Ok(None),
          ERROR_DEVICE_NOT_CONNECTED => Err(XInputOptionalFnUsageError::DeviceNotConnected),
          s => {
            trace!("Unexpected error code: {}", s);
            Err(XInputOptionalFnUsageError::UnknownError(s))
          }
        }
      }
    } else {
      Err(XInputOptionalFnUsageError::FunctionNotLoaded)
    }
  }
}

/// Defines type of battery used in device, if any.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BatteryType(pub BYTE);

impl BatteryType {
  /// Device is disconnected.
  pub const DISCONNECTED: Self = BatteryType(BATTERY_TYPE_DISCONNECTED);
  /// Device does not have battery.
  pub const WIRED: Self = BatteryType(BATTERY_TYPE_WIRED);
  /// Device has alkaline battery.
  pub const ALKALINE: Self = BatteryType(BATTERY_TYPE_ALKALINE);
  /// Device has nimh battery.
  pub const NIMH: Self = BatteryType(BATTERY_TYPE_NIMH);
  /// The battery type is not known.
  pub const UNKNOWN: Self = BatteryType(BATTERY_TYPE_UNKNOWN);
}

impl Debug for BatteryType {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    let kind: &dyn Debug = match *self {
      BatteryType::DISCONNECTED => &"DISCONNECTED",
      BatteryType::WIRED => &"WIRED",
      BatteryType::ALKALINE => &"ALKALINE",
      BatteryType::NIMH => &"NIMH",
      BatteryType::UNKNOWN => &"UNKNOWN",
      _ => &self.0,
    };

    f.debug_tuple("BatteryType").field(kind).finish()
  }
}

/// Specify how much battery is charged for devices with battery.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BatteryLevel(pub BYTE);

impl BatteryLevel {
  /// Battery is empty.
  pub const EMPTY: Self = BatteryLevel(BATTERY_LEVEL_EMPTY);
  /// Battery level is low.
  pub const LOW: Self = BatteryLevel(BATTERY_LEVEL_LOW);
  /// Battery level is medium.
  pub const MEDIUM: Self = BatteryLevel(BATTERY_LEVEL_MEDIUM);
  /// Battery is full.
  pub const FULL: Self = BatteryLevel(BATTERY_LEVEL_FULL);
}

impl Debug for BatteryLevel {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    let level: &dyn Debug = match *self {
      BatteryLevel::EMPTY => &"EMPTY",
      BatteryLevel::LOW => &"LOW",
      BatteryLevel::MEDIUM => &"MEDIUM",
      BatteryLevel::FULL => &"FULL",
      _ => &self.0,
    };

    f.debug_tuple("BatteryLevel").field(level).finish()
  }
}

/// Holds information about device's battery.
///
/// See also [XINPUT_BATTERY_INFORMATION](https://docs.microsoft.com/en-us/windows/desktop/api/xinput/ns-xinput-_xinput_battery_information).
#[derive(Debug, Copy, Clone)]
pub struct XInputBatteryInformation {
  /// Type of batter used in device, if any.
  pub battery_type: BatteryType,
  /// For devices with battery, contains battery level.
  pub battery_level: BatteryLevel,
}

impl XInputHandle {
  fn xinput_get_battery_information(
    &self,
    user_index: u32,
    dev_type: BYTE,
  ) -> Result<XInputBatteryInformation, XInputOptionalFnUsageError> {
    if user_index >= 4 {
      Err(XInputOptionalFnUsageError::InvalidControllerID)
    } else if let Some(func) = self.opt_xinput_get_battery_information {
      let mut output: XINPUT_BATTERY_INFORMATION = unsafe { ::std::mem::zeroed() };

      let return_status = unsafe { func(user_index, dev_type, &mut output) };

      match return_status {
        ERROR_SUCCESS => {
          return Ok(XInputBatteryInformation {
            battery_type: BatteryType(output.BatteryType),
            battery_level: BatteryLevel(output.BatteryLevel),
          })
        }
        s => {
          trace!("Unexpected error code: {}", s);
          Err(XInputOptionalFnUsageError::UnknownError(s))
        }
      }
    } else {
      Err(XInputOptionalFnUsageError::FunctionNotLoaded)
    }
  }

  /// Get battery type and charge level of a gamepad.
  ///
  /// See also [XInputGetBatteryInformation](https://docs.microsoft.com/en-us/windows/desktop/api/xinput/nf-xinput-xinputgetbatteryinformation)
  pub fn get_gamepad_battery_information(
    &self,
    user_index: u32,
  ) -> Result<XInputBatteryInformation, XInputOptionalFnUsageError> {
    self.xinput_get_battery_information(user_index, BATTERY_DEVTYPE_GAMEPAD)
  }

  /// Get battery type and charge level of a headset.
  ///
  /// See also [XInputGetBatteryInformation](https://docs.microsoft.com/en-us/windows/desktop/api/xinput/nf-xinput-xinputgetbatteryinformation)
  pub fn get_headset_battery_information(
    &self,
    user_index: u32,
  ) -> Result<XInputBatteryInformation, XInputOptionalFnUsageError> {
    self.xinput_get_battery_information(user_index, BATTERY_DEVTYPE_HEADSET)
  }
}

/// See `InputHandle::get_gamepad_battery_information`
#[deprecated]
pub fn xinput_get_gamepad_battery_information(
  user_index: u32,
) -> Result<XInputBatteryInformation, XInputOptionalFnUsageError> {
  match *GLOBAL_XINPUT_HANDLE {
    Ok(ref handle) => handle.get_gamepad_battery_information(user_index),
    Err(_) => Err(XInputOptionalFnUsageError::XInputNotLoaded),
  }
}

/// See `InputHandle::get_headset_battery_information`
#[deprecated]
pub fn xinput_get_headset_battery_information(
  user_index: u32,
) -> Result<XInputBatteryInformation, XInputOptionalFnUsageError> {
  match *GLOBAL_XINPUT_HANDLE {
    Ok(ref handle) => handle.get_headset_battery_information(user_index),
    Err(_) => Err(XInputOptionalFnUsageError::XInputNotLoaded),
  }
}
