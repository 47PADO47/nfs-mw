//! The radio on the main menu: it plays the front-end list while the menus are up, unless the radio is off.

use std::sync::atomic::Ordering;

use super::radio::{radio, title};

#[test]
fn the_menus_play_the_front_end_list_when_the_radio_is_on() {
    let (mut radio, mut mixer, opened) = radio();
    radio.update_held(false, true, false, true, &mut mixer);
    // Front end: A and C only.
    assert!(matches!(title(&radio), Some("A" | "C")));
    assert_eq!(opened.load(Ordering::Relaxed), 1);
}

#[test]
fn the_menus_stay_silent_when_the_radio_is_off() {
    let (mut radio, mut mixer, opened) = radio();
    radio.set_enabled(false);
    radio.update_held(false, true, false, true, &mut mixer);
    assert_eq!((title(&radio), opened.load(Ordering::Relaxed)), (None, 0));
}
