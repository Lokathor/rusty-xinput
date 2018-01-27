# rusty-gamepads

A library that lets you use all sorts of gamepads. Supports `no_std`.

Currently implemented:

* Windows
  * XInput - Dynamic loading, GetState, SetState

Hopefully there will be more soon!

# Requires Nightly because it uses const_fn

Calling certain const functions might become stable with 1.24, so starting then
the crate will become usable in stable.
