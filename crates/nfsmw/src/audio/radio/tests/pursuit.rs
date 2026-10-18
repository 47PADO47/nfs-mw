use super::super::Control;
use super::radio::*;
use std::sync::atomic::Ordering;
#[test]
fn pursuit_ownership_blocks_every_transport_entry_without_touching_history_or_pause() {
    let (mut radio, mut mixer, opened) = radio();
    radio.update(true, true, &mut mixer);
    radio.pause().unwrap();
    radio.update_held(true, false, true, true, &mut mixer);
    let before = (radio.history.clone(), radio.cursor, radio.serial);
    assert!(radio.apply(Control::Toggle, &mut mixer).is_err());
    assert!(radio.apply(Control::Next, &mut mixer).is_err());
    assert!(radio.apply(Control::Previous, &mut mixer).is_err());
    assert!(radio.play_now(2, &mut mixer).is_err());
    assert!(radio.resume().is_err());
    assert_eq!((radio.history.clone(), radio.cursor, radio.serial), before);
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    assert!(radio.paused);
    radio.update_held(true, false, false, true, &mut mixer);
    radio.resume().unwrap();
    assert!(!radio.paused);
}
