//! Player 2's engine voice needs no device to be checked: without a guest's car it stays silent and creates nothing.
use game_install::GameDir;

use super::*;
use crate::audio::Volumes;

#[test]
fn second_voice_is_silent_without_a_guest() {
    let root = std::env::temp_dir().join("nfsmw-second-voice-test");
    std::fs::create_dir_all(&root).unwrap();
    let mut audio = Audio::with_output(GameDir::open(root.clone()).unwrap(), Volumes::default(), None);
    audio.drive_second_car(None, 1.0 / 60.0);
    assert!(audio.second_car.is_none());
    assert!(audio.second_failed.is_none());
}
