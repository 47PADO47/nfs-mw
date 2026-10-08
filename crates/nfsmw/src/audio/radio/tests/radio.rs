//! The radio's decisions: when a song starts, what follows, skipping, switching off. Fakes stand in for the music
//! file and the mixer track.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

use ea_audio::mus::graph::WalkEnd;
use ea_audio::mus::{Chain, Segment};
use kira::Frame;
use kira::info::MockInfoBuilder;
use kira::sound::{Sound, SoundData};
use nfsmw_data::music::{Playability, Song};

use super::super::feeder::Started;
use super::super::stream::{StreamData, StreamHandle};
use super::super::{Backend, Mode, Player, Radio};

const RATE: u32 = 1000;
/// Frames in every fake song, so it lasts one second.
const LENGTH: usize = 1000;
/// Event of the song the fake map does not know.
const UNKNOWN: u32 = 0xDEAD;

fn song(name: &str, event: u32, playability: Playability) -> Song {
    Song { artist: "Artist".into(), title: name.into(), album: "Album".into(), event, playability }
}

/// A (FE), B (IG), C (AL), D (IG) and E (IG, not in the map).
fn songs() -> Vec<Song> {
    use Playability::*;
    vec![
        song("A", 1, FrontEnd),
        song("B", 2, InGame),
        song("C", 3, Both),
        song("D", 4, InGame),
        song("E", UNKNOWN, InGame),
    ]
}

#[derive(Default)]
struct Fake {
    opened: Arc<AtomicUsize>,
    fail: bool,
}

impl Backend for Fake {
    fn chain(&self, event: u32) -> Result<Arc<Chain>, String> {
        if event == UNKNOWN {
            return Err("no such song".into());
        }
        let segment = Segment { node: 0, stream: 0, start_ms: 0, duration_ms: 1000 };
        Ok(Arc::new(Chain { segments: vec![segment], end: WalkEnd::EndNode }))
    }

    fn open(&self, _chain: &Arc<Chain>) -> Result<Started, String> {
        self.opened.fetch_add(1, Ordering::Relaxed);
        if self.fail {
            return Err("the music file is gone".into());
        }
        let (tx, blocks) = channel();
        tx.send(vec![Frame::new(0.25, 0.25); LENGTH]).unwrap();
        Ok(Started { sample_rate: RATE, blocks })
    }
}

/// Plays into nothing; `run` advances the sounds the way the mixer would.
#[derive(Default)]
struct Mixer {
    sounds: Vec<Box<dyn Sound>>,
}

impl Player for Mixer {
    fn play(&mut self, data: StreamData) -> Result<StreamHandle, String> {
        let (sound, handle) = data.into_sound().map_err(|e| e.to_string())?;
        self.sounds.push(sound);
        Ok(handle)
    }
}

impl Mixer {
    fn run(&mut self, frames: usize) {
        let info = MockInfoBuilder::new().build();
        for sound in &mut self.sounds {
            let mut out = vec![Frame::ZERO; frames];
            sound.process(&mut out, 1.0 / f64::from(RATE), &info);
        }
        self.sounds.retain(|s| !s.finished());
    }
}

fn radio() -> (Radio, Mixer, Arc<AtomicUsize>) {
    let fake = Fake::default();
    let opened = fake.opened.clone();
    (Radio::new(songs(), Box::new(fake)), Mixer::default(), opened)
}

fn title(radio: &Radio) -> Option<&str> {
    radio.now.as_ref().map(|n| n.title.as_str())
}

#[test]
fn songs_without_a_chain_are_left_out_of_the_lists() {
    let (radio, ..) = radio();
    assert_eq!(radio.songs().len(), 5);
    // Front end: A and C. In game: B, C and D (E has no chain, so it is in neither list).
    let list = radio.list();
    let line = |t: &str| list.lines().find(|l| l.ends_with(t)).unwrap().to_owned();
    assert!(line("- A").contains("FE") && !line("- A").contains("IG"));
    assert!(!line("- B").contains("FE") && line("- B").contains("IG"));
    assert!(line("- C").contains("FE") && line("- C").contains("IG"));
    assert!(!line("- E").contains("FE") && !line("- E").contains("IG"));
}

#[test]
fn nothing_plays_until_driving_begins() {
    let (mut radio, mut mixer, opened) = radio();
    for _ in 0..3 {
        radio.update(false, true, &mut mixer);
        mixer.run(100);
    }
    assert_eq!((title(&radio), opened.load(Ordering::Relaxed)), (None, 0));
    radio.update(true, true, &mut mixer);
    assert_eq!((title(&radio), opened.load(Ordering::Relaxed)), (Some("B"), 1));
    assert_eq!(radio.now.as_ref().unwrap().artist, "Artist");
    assert!((radio.now.as_ref().unwrap().length_secs - 1.0).abs() < 1e-6);
}

#[test]
fn the_next_song_starts_when_one_ends_in_list_order_and_wraps() {
    let (mut radio, mut mixer, _) = radio();
    radio.update(true, true, &mut mixer);
    let mut heard = vec![title(&radio).unwrap().to_owned()];
    for _ in 0..4 {
        // Half way through, the same song is still on.
        mixer.run(LENGTH / 2);
        radio.update(true, true, &mut mixer);
        assert_eq!(title(&radio), Some(heard.last().unwrap().as_str()));
        // Past the end the track reports it is finished and the radio starts the next song at once.
        mixer.run(LENGTH);
        radio.update(true, true, &mut mixer);
        heard.push(title(&radio).unwrap().to_owned());
    }
    assert_eq!(heard, ["B", "C", "D", "B", "C"]);
}

#[test]
fn elapsed_time_follows_the_playback() {
    let (mut radio, mut mixer, _) = radio();
    radio.update(true, true, &mut mixer);
    mixer.run(400);
    radio.update(true, true, &mut mixer);
    let elapsed = radio.now.as_ref().unwrap().elapsed_secs;
    assert!((elapsed - 0.4).abs() < 0.02, "{elapsed}");
}

#[test]
fn leaving_the_game_stops_the_song_and_driving_again_starts_one() {
    let (mut radio, mut mixer, _) = radio();
    radio.update(true, true, &mut mixer);
    radio.update(false, true, &mut mixer);
    assert_eq!(title(&radio), None);
    mixer.run(200);
    assert!(mixer.sounds.is_empty(), "the stopped song has faded out");
    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("C"));
}

#[test]
fn off_stops_the_radio_and_on_brings_it_back() {
    let (mut radio, mut mixer, _) = radio();
    radio.update(true, true, &mut mixer);
    radio.set_enabled(false);
    assert!(!radio.enabled() && title(&radio).is_none());
    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), None);
    assert!(radio.skip(&mut mixer).is_err());
    radio.set_enabled(true);
    radio.update(true, true, &mut mixer);
    assert!(title(&radio).is_some());
}

#[test]
fn no_music_volume_means_no_music() {
    let (mut radio, mut mixer, opened) = radio();
    radio.update(true, false, &mut mixer);
    assert_eq!((title(&radio), opened.load(Ordering::Relaxed)), (None, 0));
    radio.update(true, true, &mut mixer);
    assert!(title(&radio).is_some());
    radio.update(true, false, &mut mixer);
    assert_eq!(title(&radio), None);
}

#[test]
fn skipping_stops_the_song_and_starts_the_next() {
    let (mut radio, mut mixer, _) = radio();
    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("B"));
    assert_eq!(radio.skip(&mut mixer).unwrap(), "C");
    assert_eq!(title(&radio), Some("C"));
    // The skipped song fades out while the new one plays.
    assert_eq!(mixer.sounds.len(), 2);
    mixer.run(200);
    assert_eq!(mixer.sounds.len(), 1);
    assert_eq!(radio.skip(&mut mixer).unwrap(), "D");
}

#[test]
fn a_skip_outside_the_game_plays_the_front_end_list_and_goes_on_by_itself() {
    let (mut radio, mut mixer, _) = radio();
    assert_eq!(radio.skip(&mut mixer).unwrap(), "A");
    mixer.run(LENGTH + 100);
    radio.update(false, true, &mut mixer);
    assert_eq!(title(&radio), Some("C"));
    // Driving begins: the in-game list takes over from its start.
    radio.update(true, true, &mut mixer);
    assert_eq!(title(&radio), Some("B"));
}

#[test]
fn shuffle_plays_every_in_game_song_before_a_repeat() {
    let (mut radio, mut mixer, _) = radio();
    radio.set_mode(Mode::Shuffle);
    radio.update(true, true, &mut mixer);
    let mut round = vec![title(&radio).unwrap().to_owned()];
    for _ in 0..2 {
        mixer.run(LENGTH + 10);
        radio.update(true, true, &mut mixer);
        round.push(title(&radio).unwrap().to_owned());
    }
    round.sort();
    assert_eq!(round, ["B", "C", "D"]);
}

#[test]
fn a_failing_music_file_is_reported_once_not_every_frame() {
    let fake = Fake { fail: true, ..Fake::default() };
    let opened = fake.opened.clone();
    let mut radio = Radio::new(songs(), Box::new(fake));
    let mut mixer = Mixer::default();
    for _ in 0..50 {
        radio.update(true, true, &mut mixer);
    }
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    assert_eq!(title(&radio), None);
    assert!(radio.status(true).contains("the music file is gone"));
    // A command tries again.
    assert!(radio.skip(&mut mixer).is_err());
    assert_eq!(opened.load(Ordering::Relaxed), 2);
}

#[test]
fn play_starts_a_chosen_song_and_rejects_a_bad_number() {
    let (mut radio, mut mixer, _) = radio();
    assert_eq!(radio.play_now(3, &mut mixer).unwrap(), "radio: D");
    assert_eq!(title(&radio), Some("D"));
    assert!(radio.play_now(40, &mut mixer).is_err());
}

#[test]
fn the_status_says_what_is_on() {
    let (mut radio, mut mixer, _) = radio();
    assert!(radio.status(true).starts_with("radio on, waiting for driving"));
    radio.update(true, true, &mut mixer);
    mixer.run(100);
    radio.update(true, true, &mut mixer);
    let status = radio.status(true);
    assert!(status.starts_with("radio playing Artist - B (0:00 of 0:01)"), "{status}");
    assert!(status.contains("front-end 2 songs, in-game 3 songs"), "{status}");
    radio.set_enabled(false);
    assert!(radio.status(false).starts_with("radio off (no sound device)"));
}
