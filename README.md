# rusty-xinput

Dynamically loads an xinput dll and lets you safely call the functions.

Supports `no_std`.

# Requires Nightly because it uses const_fn

Calling certain const functions will become stable with 1.24, so starting then
the crate will become usable in stable.

# If you want to use other controller types

If you have a controller that doesn't have an Xinput driver it probably uses
DirectInput instead. The DirectInput system isn't bound within the `winapi`
crate because I'm too lazy to go make that PR.

Instead I can suggest you two options:

* You can tell your users to try the [Controller
  Emulator](https://github.com/x360ce/x360ce) program. It lets you setup the
  device you want and then spits out an XInput DLL to use that will read the
  desired device. Place the generated DLL into the same directory as your
  executable under the name `xinput9_1_0.dll` and it'll get loaded instead of
  the system level version. See their site for more info.

* You can use the [multiinput](https://crates.io/crates/multiinput) crate, which
  uses the rawinput system, which will also include things like DirectInput
  devices. I don't know the guy that makes it and I haven't used it myself, I
  just found it on crates.io and it's the only other gamepad library that's even
  been updated in the past year.
