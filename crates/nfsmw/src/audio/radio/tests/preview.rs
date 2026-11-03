//! Preview owns a temporary music stream and returns the prior transport without consuming playlist/history entries.

use std::sync::atomic::Ordering;

use super::radio::{radio, title};

#[test]
fn jukebox_preview_restores_the_menu_song_and_does_not_consume_history() {
    let (mut radio, mut mixer, _) = radio();
    radio.update_held(false, true, false, true, &mut mixer);
    mixer.run(100);
    radio.update_held(false, true, false, true, &mut mixer);
    let before = radio.now.clone().unwrap();
    let history = radio.history.clone();
    let cursor = radio.cursor;
    radio.preview(1, &mut mixer).unwrap();
    assert_eq!(title(&radio), Some("B"), "preview can play a driving-only track in the jukebox");
    assert_eq!(radio.history, history);
    assert_eq!(radio.cursor, cursor);
    mixer.run(200);
    radio.update_held(false, true, false, true, &mut mixer);
    radio.end_preview();
    assert_eq!(title(&radio), Some(before.title.as_str()));
    assert_eq!(radio.history, history);
    assert_eq!(radio.cursor, cursor);
    mixer.run(100);
    radio.update_held(false, true, false, true, &mut mixer);
    let resumed = radio.now.as_ref().unwrap().elapsed_secs;
    assert!(resumed >= before.elapsed_secs && resumed < 0.4, "prior song resumes near its held position: {resumed}");
}

#[test]
fn jukebox_preview_replacement_and_end_do_not_autoplay_another_track() {
    let (mut radio, mut mixer, opened) = radio();
    radio.update_held(false, true, false, true, &mut mixer);
    radio.preview(1, &mut mixer).unwrap();
    radio.preview(3, &mut mixer).unwrap();
    assert_eq!(title(&radio), Some("D"));
    mixer.run(2000);
    radio.update_held(false, true, false, true, &mut mixer);
    assert_eq!(title(&radio), None);
    assert_eq!(opened.load(Ordering::Relaxed), 3);
    radio.end_preview();
    assert_eq!(title(&radio), Some("A"), "replacement keeps the original snapshot");
}

#[test]
fn jukebox_preview_preserves_disabled_and_paused_radio_states() {
    let (mut radio, mut mixer, _) = radio();
    radio.set_enabled(false);
    radio.preview(1, &mut mixer).unwrap();
    assert!(!radio.enabled());
    assert_eq!(title(&radio), Some("B"));
    radio.end_preview();
    assert_eq!(title(&radio), None);
    assert!(!radio.enabled());
    radio.set_enabled(true);
    radio.update_held(false, true, false, true, &mut mixer);
    radio.pause().unwrap();
    radio.preview(1, &mut mixer).unwrap();
    radio.end_preview();
    assert!(radio.paused);
    assert!(radio.now.as_ref().unwrap().paused);
}

#[test]
fn jukebox_preview_rejects_missing_tracks_without_interrupting_the_menu_song() {
    let (mut radio, mut mixer, _) = radio();
    radio.update_held(false, true, false, true, &mut mixer);
    let before = radio.now.clone();
    assert!(radio.preview(99, &mut mixer).is_err());
    assert!(radio.preview(4, &mut mixer).is_err(), "known song without a playable chain");
    assert_eq!(radio.now, before);
    assert!(radio.preview.is_none());
}
