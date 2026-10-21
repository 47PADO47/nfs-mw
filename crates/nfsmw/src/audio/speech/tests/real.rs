//! Against the user's own install: the events of the attribute database meet the recordings of `copspeech.big`.
//!
//! `NFSMW_GAME_DIR="D:/..." cargo test --release -p nfsmw speech::tests::real -- --ignored --nocapture`

use blackbox_attrib::Database;
use ea_audio::speech::Params;
use game_install::GameDir;
use nfsmw_data::speech::{SpeechEvent, events, tune};

use crate::audio::speech::random::Rng;
use crate::audio::speech::{Dispatcher, Files, Lines, Request, Voice, params};

fn install() -> Option<GameDir> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(GameDir::open(std::path::PathBuf::from(dir)).expect("NFSMW_GAME_DIR cannot be indexed"))
}

fn load() -> Option<(Vec<SpeechEvent>, Dispatcher, Files)> {
    let dir = install()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let list = events(&db);
    let files = Files::open(
        &dir.read("SOUND/SPEECH/copspeech.idx").unwrap(),
        &dir.read("SOUND/SPEECH/copspeech.evt").unwrap(),
        dir.resolve("SOUND/SPEECH/copspeech.big").unwrap(),
        Rng::new(1),
    )
    .unwrap();
    Some((list.clone(), Dispatcher::new(list, tune(&db), Rng::new(2)), files))
}

/// A voice that decodes the lines it is asked for, as the game does, and says how long each is.
struct Decoding<'a> {
    files: &'a mut Files,
    busy: bool,
    said: Vec<(String, f32)>,
}

impl Voice for Decoding<'_> {
    fn busy(&self) -> bool {
        self.busy
    }

    fn play(&mut self, event: &SpeechEvent, request: &Request, _cut: bool) -> bool {
        let numbered = params::lib_params(&event.name, request.speaker, &request.params);
        let Ok(pcm) = self.files.take(event.id, request.speaker, &numbered) else { return false };
        self.said.push((event.name.clone(), pcm.frames() as f32 / pcm.sample_rate as f32));
        self.busy = true;
        true
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_events_with_recordings_of_their_own_are_the_28_the_banks_name() {
    let Some((list, _, files)) = load() else { return };
    let voiced: Vec<&SpeechEvent> = list.iter().filter(|e| files.has(e.id)).collect();
    for e in &voiced {
        println!("{:3} {}", e.id, e.name);
    }
    assert_eq!(voiced.len(), 28);
    assert_eq!(files.summary(), "579 banks, 13562 takes, 139 sentences");
    for name in ["arrest_disparrestreply", "anytimeevents_bailoutdeny", "anytimeevents_weatherreport"] {
        assert!(voiced.iter().any(|e| e.name == name), "{name}");
    }
    assert!(!voiced.iter().any(|e| e.name == "acknowledge"), "a sentence, not a phrase");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_recorded_event_decodes_to_a_line_of_plausible_length() {
    let Some((list, _, mut files)) = load() else { return };
    let voiced: Vec<&SpeechEvent> = list.iter().filter(|e| files.has(e.id)).collect();
    for e in voiced {
        for speaker in [0, 1, 4, 9] {
            let pcm = files
                .take(e.id, speaker, &Params::new())
                .unwrap_or_else(|err| panic!("{} for {speaker}: {err}", e.name));
            let secs = pcm.frames() as f32 / pcm.sample_rate as f32;
            assert_eq!(pcm.sample_rate, 24000, "{}", e.name);
            assert!((0.2..30.0).contains(&secs), "{} is {secs} s", e.name);
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_dispatcher_says_recorded_events_and_drops_sentences_without_rules() {
    let Some((list, mut dispatcher, mut files)) = load() else { return };
    let weather = list.iter().find(|e| e.name == "anytimeevents_weatherreport").unwrap().id;
    let acknowledge = list.iter().find(|e| e.name == "acknowledge").unwrap().id;
    dispatcher.request(acknowledge, Request::default());
    dispatcher.request(weather, Request::default());
    let mut voice = Decoding { files: &mut files, busy: false, said: Vec::new() };
    // The weather report needs a speed above 30 mph.
    let context = crate::audio::speech::Context { heat: 0, player_speed: 50.0 };
    for _ in 0..20 {
        dispatcher.update(0.1, context, &mut voice);
    }
    let names: Vec<&str> = voice.said.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["anytimeevents_weatherreport"]);
    println!("{:?}", voice.said);
    // `acknowledge` follows one of four other events and none was said, so it keeps waiting for its turn.
    assert_eq!(dispatcher.waiting(), 1);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_pursuit_lines_are_said_as_sentences() {
    let Some((_, _, mut files)) = load() else { return };
    // The pursuit cues of the chase (`chase/cues.rs`): sentences with no banks of their own, which are joined from
    // their phrases. Each must come out with audio.
    for (name, id) in [
        ("setup_initpursuit", 61),
        ("lostvisual", 106),
        ("regainvisual", 107),
        ("lostsuspect", 108),
        ("disptimeexpired", 180),
    ] {
        let pcm = files.take(id, 0, &Params::new()).unwrap_or_else(|e| panic!("{name} (SpeechID {id}): {e}"));
        println!("{name}: {:.2} s", pcm.frames() as f32 / pcm.sample_rate as f32);
        assert!(pcm.frames() > 0, "{name} is silent");
    }
}
