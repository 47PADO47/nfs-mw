//! The radio with the player's own tracks: which track plays next, replacing or mixed with the songs, and what happens
//! to a track that will not open. The tracks are real WAV files in a temp folder; the songs are the fakes of `radio.rs`.
//! Songs in the in-game list, in order: B, C, D. Custom tracks in a folder, in name order: tune1, tune2.

use super::super::Radio;
use super::super::custom::Library;
use super::super::custom::test_files::{folder, write_wav};
use super::super::state::NO_SONG;
use super::radio::{Mixer, radio, title};
use crate::settings::{CustomMode, CustomMusic, CustomSources};

/// The sample rate of the test tracks: one second is 1000 frames.
const RATE: u32 = 1000;

/// A radio over the fake songs with the tracks of `dir` as the custom tracks, in `mode`.
fn radio_with(dir: &std::path::Path, mode: CustomMode) -> (Radio, Mixer) {
    let (mut radio, mixer, _) = radio();
    let config = CustomMusic {
        enabled: true,
        mode,
        global: CustomSources { folders: vec![dir.to_path_buf()], playlists: Vec::new() },
        ..CustomMusic::default()
    };
    radio.set_custom(&config, Library::build(&config));
    (radio, mixer)
}

#[test]
fn replace_plays_the_custom_tracks_in_place_of_the_songs() {
    let dir = folder("replace");
    write_wav(&dir, "tune1", RATE, 1, RATE);
    write_wav(&dir, "tune2", RATE, 1, RATE);
    let (mut radio, mut mixer) = radio_with(&dir, CustomMode::Replace);

    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("tune1"));
    assert_eq!(radio.now.as_ref().map(|now| now.song), Some(NO_SONG));
    radio.skip(&mut mixer).unwrap();
    assert_eq!(title(&radio), Some("tune2"));
    radio.skip(&mut mixer).unwrap();
    assert_eq!(title(&radio), Some("tune1"), "the list wraps to its start");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mix_alternates_one_song_and_one_custom_track_starting_with_the_song() {
    let dir = folder("mix");
    write_wav(&dir, "tune1", RATE, 1, RATE);
    write_wav(&dir, "tune2", RATE, 1, RATE);
    let (mut radio, mut mixer) = radio_with(&dir, CustomMode::Mix);

    let mut heard = Vec::new();
    radio.update(true, true, &mut mixer);
    heard.push(title(&radio).unwrap().to_owned());
    for _ in 0..5 {
        radio.skip(&mut mixer).unwrap();
        heard.push(title(&radio).unwrap().to_owned());
    }
    assert_eq!(heard, ["B", "tune1", "C", "tune2", "D", "tune1"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn previous_walks_back_across_songs_and_custom_tracks() {
    let dir = folder("previous");
    write_wav(&dir, "tune1", RATE, 1, RATE);
    let (mut radio, mut mixer) = radio_with(&dir, CustomMode::Mix);

    radio.update(true, true, &mut mixer);
    radio.skip(&mut mixer).unwrap();
    radio.skip(&mut mixer).unwrap();
    assert_eq!(title(&radio), Some("C"));
    // Nothing has played for three seconds yet, so `previous` goes back one track: the custom one.
    radio.previous(&mut mixer).unwrap();
    assert_eq!(title(&radio), Some("tune1"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_track_that_will_not_open_is_skipped_for_the_next() {
    let dir = folder("broken");
    std::fs::write(dir.join("a-broken.wav"), b"this is not audio at all").unwrap();
    write_wav(&dir, "b-good", RATE, 1, RATE);
    let (mut radio, mut mixer) = radio_with(&dir, CustomMode::Replace);

    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("b-good"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_skipped_custom_track_fades_out_over_the_default_five_seconds() {
    let dir = folder("fade");
    // Ten seconds each, so neither track ends during the test.
    write_wav(&dir, "tune1", RATE, 1, 10 * RATE);
    write_wav(&dir, "tune2", RATE, 1, 10 * RATE);
    let config = CustomMusic {
        enabled: true,
        mode: CustomMode::Replace,
        global: CustomSources { folders: vec![dir.clone()], playlists: Vec::new() },
        ..CustomMusic::default()
    };
    let (mut radio, mut mixer, _) = radio();
    radio.set_custom(&config, Library::build(&config));
    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("tune1"));
    radio.skip(&mut mixer).unwrap();
    assert_eq!(title(&radio), Some("tune2"));
    assert_eq!(mixer.sounds.len(), 2, "the skipped track fades while the new one plays");
    // 80 ms would have ended the skipped track by now; the default 5000 ms has not.
    mixer.run(1000);
    assert_eq!(mixer.sounds.len(), 2, "still fading after one second");
    // Up to 10 s of frames: the fade is 5 s, and a stream that is waiting for its decoder does not fade meanwhile.
    for _ in 0..100 {
        if mixer.sounds.len() == 1 {
            break;
        }
        // This virtual mixer can outrun the real decoder thread. Give it a scheduling
        // opportunity before advancing another audio block; waiting streams do not fade.
        std::thread::sleep(std::time::Duration::from_millis(1));
        mixer.run(100);
    }
    assert_eq!(mixer.sounds.len(), 1, "the skipped track has faded out");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_config_with_no_tracks_leaves_the_songs_alone() {
    let dir = folder("empty");
    let (mut radio, mut mixer) = radio_with(&dir, CustomMode::Replace);

    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("B"));
    let _ = std::fs::remove_dir_all(dir);
}
