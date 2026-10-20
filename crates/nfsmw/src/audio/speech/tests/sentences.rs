//! The speech sentences with the game's names for the parameters, against the user's own install.
//!
//! `NFSMW_GAME_DIR="D:/..." cargo test --release -p nfsmw speech::tests::sentences -- --ignored --nocapture`

use blackbox_attrib::Database;
use ea_audio::speech::{Assembler, Chosen, Repeats, Sentences, SpeechIndex};
use game_install::GameDir;
use nfsmw_data::speech::{SpeechEvent, events};

use crate::audio::speech::params::{self, Param, SpeechParams, signature};
use crate::audio::speech::random::Rng;
use crate::audio::speech::{Files, Lines};

struct World {
    events: Vec<SpeechEvent>,
    sentences: Sentences,
    index: SpeechIndex,
    files: Files,
}

fn load() -> Option<World> {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?)).ok()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let idx = dir.read("SOUND/SPEECH/copspeech.idx").unwrap();
    let evt = dir.read("SOUND/SPEECH/copspeech.evt").unwrap();
    let big = dir.resolve("SOUND/SPEECH/copspeech.big").unwrap();
    Some(World {
        events: events(&db),
        sentences: Sentences::parse(&evt).unwrap(),
        index: SpeechIndex::parse(&idx).unwrap(),
        files: Files::open(&idx, &evt, big, Rng::new(3)).unwrap(),
    })
}

impl World {
    fn id(&self, name: &str) -> u16 {
        let event = self.events.iter().find(|e| e.name == name).unwrap_or_else(|| panic!("no event {name}"));
        event.id as u16
    }
}

/// The single-bit values of a parameter.
fn bits(param: Param) -> Vec<u32> {
    (0..32).map(|b| 1u32 << b).filter(|v| v & param.choices() != 0).collect()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_argument_lists_agree_with_the_tests_of_every_sentence() {
    let Some(world) = load() else { return };
    let mut checked = 0;
    let mut done = std::collections::HashSet::new();
    for event in &world.events {
        // `collworld_spin` carries the number of `collisionworld` (98) in the attribute database: one record, checked
        // under the first name.
        if !done.insert(event.id) {
            continue;
        }
        let Some(sentence) = u16::try_from(event.id).ok().and_then(|id| world.sentences.sentence(id)) else { continue };
        let Some((has_speaker, fields)) = signature(&event.name) else {
            panic!("{} has a sentence but no argument list", event.name);
        };
        let first = if has_speaker { 2 } else { 1 };
        for test in sentence.blocks.iter().flat_map(|b| &b.entries).flat_map(|e| &e.tests) {
            if test.param == 0 {
                continue;
            }
            let field =
                fields.get(usize::from(test.param) - first).unwrap_or_else(|| panic!("{}: {test:?}", event.name));
            // The test spans the parameter's choices, or a part of them (a sentence version for some choices only).
            assert!(
                test.mask != 0 && test.mask & !field.choices() == 0,
                "{}: {} {:#x}",
                event.name,
                field.name(),
                test.mask
            );
            checked += 1;
        }
    }
    assert!(checked > 150, "{checked}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_value_of_a_parameter_chooses_the_take_that_is_recorded_for_it() {
    let Some(mut world) = load() else { return };
    // Pursuit lines and the parameter that changes what they say.
    let cases = [
        ("setup_vehiclereport", Param::CarColor),
        ("setup_vehiclereport", Param::CarType),
        ("setup_vehiclereport", Param::Speed),
        ("setup_vehiclereporttag", Param::CarColor),
        ("setup_locationreport", Param::Direction),
        ("setup_locationreport", Param::LocationRegion),
        ("setup_attmptvehstp", Param::PursuitType),
        ("setup_attmptvehstp", Param::SpeakerBattalion),
        ("setup_primaryengage", Param::SpeakerBattalion),
        ("anytimeevents_collisionworld", Param::WorldObjectType),
        ("anytimeevents_heatjump", Param::HeatLevel),
        ("setup_selfstrategy", Param::SelfStrategyType),
        ("rollingstrategy_initstrategy", Param::RollingStrategyType),
        ("backup_callforbu", Param::BackupType),
        ("backup_dispbueta", Param::DispBackupEta),
        ("anytimeevents_disppursuitescalation", Param::Direction),
        ("anytimeevents_bailout", Param::BailoutType),
        ("helispecific_helihazardalert", Param::HeliHazardAlertType),
    ];
    let assembler = Assembler::new(&world.sentences, &world.index);
    for (name, param) in cases {
        let id = world.id(name);
        let (has_speaker, fields) = signature(name).unwrap();
        let number = fields.iter().position(|&f| f == param).unwrap() as u8 + if has_speaker { 2 } else { 1 };
        let sentence = world.sentences.sentence(id).unwrap();
        // The entries that test this parameter, and which test of the entry it is.
        let tested: Vec<(u16, usize)> = sentence
            .blocks
            .iter()
            .flat_map(|b| &b.entries)
            .filter_map(|e| Some((e.phrase, e.tests.iter().position(|t| t.param == number)?)))
            .collect();
        assert!(!tested.is_empty(), "{name} never tests {}", param.name());
        let mut seen = std::collections::BTreeSet::new();
        for value in bits(param) {
            // The speaker is a voice that recorded the sentence; the other parameters are left open.
            let params = SpeechParams::new().with(param, value);
            let numbered = params::lib_params(name, 3, &params);
            for seed in 0..6 {
                let mut rng = Rng::new(seed);
                let Some(parts) = assembler.assemble(id, &numbered, &mut Repeats::default(), &mut rng) else {
                    continue;
                };
                for Chosen { bank, take } in parts {
                    let header = &world.index.banks()[bank].header;
                    let Some(&(_, at)) = tested.iter().find(|(phrase, _)| *phrase == header.event) else { continue };
                    let bit = header.selectors_of(take)[at];
                    assert_eq!(
                        1u32 << bit,
                        value,
                        "{name} {}={value:#x}: phrase {} take {take}",
                        param.name(),
                        header.event
                    );
                    seen.insert(bit);
                }
            }
        }
        println!("{name} {}: {} values have recordings", param.name(), seen.len());
        assert!(seen.len() >= 2, "{name}: {} gives {seen:?}", param.name());
    }
    // Decode one of them to be sure the takes are audio: the report of a red car, heading east, at 140.
    let params = SpeechParams::new()
        .with(Param::CarColor, params::values::car_color::RED)
        .with(Param::CarType, params::values::car_type::PORSCHE)
        .with(Param::Speed, params::speed_for(140.0))
        .with(Param::Measurement, params::values::measurement::IMPERIAL_ONLY);
    let id = u32::from(world.id("setup_vehiclereport"));
    let pcm = world.files.take(id, 3, &params::lib_params("setup_vehiclereport", 3, &params)).unwrap();
    assert!((1.0..20.0).contains(&(pcm.frames() as f32 / pcm.sample_rate as f32)));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_pursuit_line_is_a_sentence_of_plausible_length_not_all_its_variants() {
    let Some(mut world) = load() else { return };
    let mut rng = Rng::new(9);
    let (mut lines, mut longest) = (0, 0.0f32);
    let names: Vec<(String, u32)> = world.events.iter().map(|e| (e.name.clone(), e.id)).collect();
    for (name, id) in names {
        let Some((has_speaker, fields)) = signature(&name) else { continue };
        let Some(sentence) = u16::try_from(id).ok().and_then(|i| world.sentences.sentence(i)) else { continue };
        let widest = sentence.blocks.iter().map(|b| b.entries.len()).max().unwrap();
        // Everything the sentence could say: one take of every bank of every phrase of every version is a floor.
        let variants: usize = sentence
            .blocks
            .iter()
            .flat_map(|b| &b.entries)
            .map(|e| world.index.banks_for(e.phrase).map(|b| b.header.takes()).sum::<usize>())
            .sum();
        for _ in 0..12 {
            let mut params = SpeechParams::new();
            for &field in fields {
                let choices = bits(field);
                params.set(field, choices[rng.below(choices.len())]);
            }
            let speaker = if has_speaker { 1 + rng.below(9) as u16 } else { 0 };
            let numbered = params::lib_params(&name, speaker, &params);
            let mut check = Rng::new(lines as u64);
            let assembler = Assembler::new(&world.sentences, &world.index);
            let Some(parts) = assembler.assemble(id as u16, &numbered, &mut Repeats::default(), &mut check) else {
                continue;
            };
            // One take per phrase of a version, never every variant.
            assert!(
                parts.len() <= widest,
                "{name}: {} takes for at most {widest} phrases ({variants} variants)",
                parts.len()
            );
            let Ok(pcm) = world.files.take(id, speaker, &numbered) else { continue };
            let secs = pcm.frames() as f32 / pcm.sample_rate as f32;
            lines += 1;
            longest = longest.max(secs);
            // The longest single takes are the phone calls of the cell-call event, up to about 30 s.
            assert!(secs < 40.0 * widest as f32, "{name}: {secs} s for at most {widest} takes");
            assert!(secs > 0.05, "{name} is silent");
        }
    }
    println!("{lines} lines said, the longest {longest:.1} s");
    assert!(lines > 1000, "{lines}");
}
