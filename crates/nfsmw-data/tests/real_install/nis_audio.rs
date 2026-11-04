//! The NIS audio streams: the take of each scene's stream is found in the install and decodes to PCM, and each arrest
//! plays its own take of the busted bank (`docs/specs/nis-events.md` section 7.9).

use std::collections::HashSet;

use ea_audio::ReadAt;
use ea_audio::schl::StreamReader;
use ea_audio::speech::{Sentences, SpeechIndex};
use game_install::GameDir;
use nfsmw_data::nis::NisBundle;
use nfsmw_data::sound::{NIS_SCENES, NisBranch, nis_branch, nis_take};

use crate::install;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const EVENTS: &str = "SOUND/STREAMS/NISAudio.evt";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";

/// The phrase (bank event) of each branch's bank: start, blacklist and end.
const START_PHRASE: u16 = 0x4023;
const BLACKLIST_PHRASE: u16 = 0x4025;
const END_PHRASE: u16 = 0x4024;

/// The arrests that have a bundle, with their choice in `NIS_Scene_End` (a bit index).
const ARRESTS: [(&str, u8); 13] = [
    ("ArrestF02", 7),
    ("ArrestF06", 8),
    ("ArrestF07", 9),
    ("ArrestF14", 10),
    ("ArrestF18", 14),
    ("ArrestF23", 11),
    ("ArrestM01", 0),
    ("ArrestM04", 1),
    ("ArrestM06", 2),
    ("ArrestM07", 15),
    ("ArrestM14", 3),
    ("ArrestM16", 4),
    ("ArrestM19", 5),
];

/// `NISAudio.big`, read on demand: the file is about 184 MB.
struct Stream<'a> {
    dir: &'a GameDir,
    len: u64,
}

impl ReadAt for Stream<'_> {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        let bytes = self.dir.read_range(STREAM, offset, buf.len()).unwrap_or_default();
        buf[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
    }
}

fn tables(dir: &GameDir) -> (SpeechIndex, Sentences) {
    let index = SpeechIndex::parse(&dir.read(INDEX).expect("NISAudio.idx")).expect("NISAudio.idx parses");
    let events = Sentences::parse(&dir.read(EVENTS).expect("NISAudio.evt")).expect("NISAudio.evt parses");
    (index, events)
}

fn stream(dir: &GameDir) -> Stream<'_> {
    let len = std::fs::metadata(dir.resolve(STREAM).expect("NISAudio.big")).expect("NISAudio.big metadata").len();
    Stream { dir, len }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_nis_scenes_find_and_decode_their_audio_takes() {
    let Some(dir) = install() else { return };
    let (index, events) = tables(&dir);
    let big = stream(&dir);
    for scene in ["IntroNis05", "IntroNisBL02", "EndingNis04", "ArrestF02"] {
        let take = nis_take(&index, &events, scene).unwrap_or_else(|| panic!("{scene}: no take"));
        let pcm = index.banks()[take.bank].decode(&big, take.take).unwrap_or_else(|e| panic!("{scene}: {e:?}"));
        assert!(pcm.frames() > 0 && !pcm.samples.is_empty(), "{scene}: empty take");
        println!(
            "{scene}: bank {} take {}, {:.2} s, {} Hz, {} ch",
            take.bank,
            take.take,
            pcm.duration_secs(),
            pcm.sample_rate,
            pcm.channels
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn each_arrest_plays_its_own_busted_take_and_it_covers_the_scene() {
    let Some(dir) = install() else { return };
    let (index, events) = tables(&dir);
    let big = stream(&dir);
    let mut takes = HashSet::new();
    for (scene, choice) in ARRESTS {
        let take = nis_take(&index, &events, scene).unwrap_or_else(|| panic!("{scene}: no take"));
        let b = format!("{scene}b");
        assert_eq!(nis_take(&index, &events, &b), Some(take), "{b} plays the take of {scene}");
        let bank = &index.banks()[take.bank];
        assert_eq!(bank.header.event, END_PHRASE, "{scene}: the busted bank");
        assert_eq!(bank.header.selectors_of(take.take), [choice, 13, 0], "{scene}: its choice, track 0, complete");
        assert!(takes.insert(take), "{scene}: a take of its own");
        let offset = bank.take_offset(take.take).expect("the take's offset");
        let reader = StreamReader::open(&big, offset).unwrap_or_else(|e| panic!("{scene}: {e:?}"));
        let seconds = f64::from(reader.header().sample_count) / f64::from(reader.header().sample_rate);
        let bundle = NisBundle::load(&dir, scene).unwrap_or_else(|e| panic!("{scene}: {e:#}"));
        let length = f64::from(bundle.scene.vanish_frame) / 30.0;
        println!("{scene}: take {}, {seconds:.2} s for a scene of {length:.2} s", take.take);
        // The takes run 0.05 to 1.8 s past the scene's end frame in the install.
        assert!((length..=length + 2.0).contains(&seconds), "{scene}: {seconds:.2} s for {length:.2} s");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_table_scene_takes_a_take_of_its_branch_bank() {
    let Some(dir) = install() else { return };
    let (index, events) = tables(&dir);
    let silent = ["Flythrough", "GarageEnter", "GarageExit", "OPMRivalIntro", "IntroNisBL01"];
    for scene in NIS_SCENES {
        let take = nis_take(&index, &events, scene);
        if silent.contains(&scene) {
            assert_eq!(take, None, "{scene}: the install has no take for its choice");
            continue;
        }
        let take = take.unwrap_or_else(|| panic!("{scene}: no take"));
        let phrase = match nis_branch(scene).expect("a table scene") {
            NisBranch::Start => START_PHRASE,
            NisBranch::Blacklist => BLACKLIST_PHRASE,
            NisBranch::End => END_PHRASE,
        };
        let bank = &index.banks()[take.bank].header;
        assert_eq!(bank.event, phrase, "{scene}: the bank of its branch");
        assert_eq!(bank.selectors_of(take.take)[1..], [13, 0], "{scene}: track 0, complete section");
    }
}
