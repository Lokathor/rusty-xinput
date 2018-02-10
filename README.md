# rusty-xinput

Dynamically loads an xinput dll and lets you safely call the functions.

Supports `no_std`.

# Requires Nightly because it uses const_fn

Calling certain const functions might become stable with 1.24, so starting then
the crate will become usable in stable.
