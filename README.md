[![License:0BSD](https://img.shields.io/badge/License-0BSD-brightgreen.svg)](https://opensource.org/licenses/FPL-1.0.0)
[![CratesIO](https://img.shields.io/crates/v/rusty-xinput.svg)](https://crates.io/crates/rusty-xinput)
[![DocsRS](https://docs.rs/rusty-xinput/badge.svg)](https://docs.rs/rusty-xinput/)
[![Appveyor](https://ci.appveyor.com/api/projects/status/2nhvh047mrv8plen?svg=true)](https://ci.appveyor.com/project/Lokathor/rusty-xinput)

# rusty-xinput

Dynamically loads an xinput dll and lets you safely call the functions.

# If you want to use other controller types

If you have a controller that doesn't have an XInput driver it probably uses
DirectInput instead. The DirectInput system isn't bound within the `winapi`
crate because I'm too lazy to go make that PR and no one else cares.

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
