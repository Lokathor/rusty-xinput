#[macro_use]
extern crate log;

extern crate simple_logger;

extern crate rusty_gamepads;
use rusty_gamepads::*;

fn main() {
  simple_logger::init().unwrap();

  dynamic_load_xinput().unwrap();

  loop {
    ::std::thread::sleep(::std::time::Duration::from_millis(16));
    match xinput_get_state(0) {
      None => {
        error!("Controller 0 not detected!");
        break;
      }
      Some(state) => {
        if state.east_button() {
          break;
        } else {
          println!(
            "l:{:?}, r:{:?}",
            state.left_stick_normalized(),
            state.right_stick_normalized()
          );
        }
      }
    }
  }
}
