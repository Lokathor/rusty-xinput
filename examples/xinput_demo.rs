#[macro_use]
extern crate log;

extern crate simple_logger;

extern crate rusty_xinput;
use rusty_xinput::*;

fn main() {
  simple_logger::init().unwrap();

  // If we fail to load the rest of the demo clearly can't run, so we'll just do
  // an unwrap here.
  dynamic_load_xinput().unwrap();

  // Quick rumble test. Note that the controller might not _have_ rumble.
  trace!("rumble on:{:?}", xinput_set_state(0, 1000, 1000));
  ::std::thread::sleep(::std::time::Duration::from_millis(160));
  trace!("rumble off:{:?}", xinput_set_state(0, 0, 0));

  // Show stick values, loop until the button is pressed to stop.
  loop {
    ::std::thread::sleep(::std::time::Duration::from_millis(16));
    match xinput_get_state(0) {
      Err(e) => {
        error!("xinput_get_state error: {:?}", e);
        break;
      }
      Ok(state) => {
        if state.east_button() {
          break;
        } else {
          info!(
            "l:{:?}, r:{:?}",
            state.left_stick_normalized(),
            state.right_stick_normalized()
          );
        }
      }
    }
  }
}
