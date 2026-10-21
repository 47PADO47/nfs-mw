//! The speech sentences against the user's own install; every test is ignored without `NFSMW_GAME_DIR`.
//!
//! Run with `cargo test --release -p ea-audio --test real_speech -- --ignored --nocapture`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ea_audio::speech::{Assembler, Params, Random, Repeats, Sentences, SpeechIndex, Voice};

fn load() -> Option<(Sentences, SpeechIndex)> {
    let dir = PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?).join("SOUND/SPEECH");
    let sentences = Sentences::parse(&std::fs::read(dir.join("copspeech.evt")).unwrap()).unwrap();
    let index = SpeechIndex::parse(&std::fs::read(dir.join("copspeech.idx")).unwrap()).unwrap();
    Some((sentences, index))
}

struct Lcg(u64);

impl Random for Lcg {
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_database_holds_139_sentences_and_every_entry_has_its_banks() {
    let Some((sentences, index)) = load() else { return };
    assert_eq!(sentences.len(), 139);
    let mut entries = 0;
    for event in sentences.events() {
        let sentence = sentences.sentence(event).unwrap();
        assert!(!sentence.blocks.is_empty(), "event {event}");
        assert!(sentence.blocks.len() <= 2, "event {event}");
        for block in &sentence.blocks {
            assert_eq!(block.chance, 100, "event {event}");
            assert!(!block.entries.is_empty() && block.entries.len() <= 12, "event {event}");
            for entry in &block.entries {
                entries += 1;
                let banks: Vec<_> = index.banks_for(entry.phrase).collect();
                assert!(!banks.is_empty(), "event {event}: phrase {} has no bank", entry.phrase);
                // One test per selector of the phrase's takes, in every bank of the phrase.
                for bank in banks {
                    assert_eq!(bank.header.selector_count, entry.tests.len(), "event {event}: phrase {}", entry.phrase);
                }
                // A phrase said in a speaker's voice has no bank for every voice; a phrase for any voice has one.
                let for_any = index.banks_for(entry.phrase).all(|b| b.header.speaker == 0xFFFF);
                assert_eq!(entry.voice == Voice::Any, for_any, "event {event}: phrase {}", entry.phrase);
            }
        }
    }
    assert_eq!(entries, 223);
}

/// A parameter value for the nth bit of a test mask: one bit, as a game passes.
fn pick_bit(mask: u32, rng: &mut Lcg) -> u32 {
    let bits: Vec<u32> = (0..32).filter(|b| mask & (1 << b) != 0).collect();
    1 << bits[rng.below(bits.len())]
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_sentence_is_said_with_one_take_per_entry_whatever_the_parameters() {
    let Some((sentences, index)) = load() else { return };
    let assembler = Assembler::new(&sentences, &index);
    let mut rng = Lcg(7);
    let mut repeats = Repeats::default();
    let (mut said, mut missed) = (0, Vec::new());
    let mut per: std::collections::BTreeMap<u16, u32> = Default::default();
    for event in sentences.events() {
        let sentence = sentences.sentence(event).unwrap();
        let widest = sentence.blocks.iter().map(|b| b.entries.len()).max().unwrap();
        let narrowest = sentence.blocks.iter().map(|b| b.entries.len()).min().unwrap();
        for speaker in 1..=9u32 {
            for _ in 0..8 {
                // Parameter 1 is the speaker for sentences that take a voice, the first argument otherwise.
                let mut params = Params::new().with(1, speaker);
                for test in sentence.blocks.iter().flat_map(|b| &b.entries).flat_map(|e| &e.tests) {
                    if test.param != 0 {
                        params.set(test.param, pick_bit(test.mask, &mut rng));
                    }
                }
                let Some(parts) = assembler.assemble(event, &params, &mut repeats, &mut rng) else {
                    // A parameter combination no version can say: a take is missing for it.
                    missed.push(event);
                    *per.entry(event).or_insert(0) += 1;
                    continue;
                };
                said += 1;
                assert!((narrowest..=widest).contains(&parts.len()), "event {event}: {} takes", parts.len());
                for c in &parts {
                    assert!(c.take < index.banks()[c.bank].header.takes(), "event {event}");
                }
            }
        }
    }
    println!("misses per event (of 72 tries): {per:?}");
    missed.sort_unstable();
    missed.dedup();
    println!("{said} sentences said; events with parameter combinations nobody recorded: {missed:?}");
    assert!(said > 8000, "{said}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn different_parameters_choose_different_takes() {
    let Some((sentences, index)) = load() else { return };
    let assembler = Assembler::new(&sentences, &index);
    // The sentence of event 155 says the car's colour and type (parameters 2 and 3) and then the speed (4, 5): its
    // first phrase has one selector per colour and per car type.
    let firsts: Vec<u16> = sentences.sentence(155).unwrap().blocks.iter().map(|b| b.entries[0].phrase).collect();
    let selectors_of = |colour_bit: u32, car_bit: u32, seed: u64| {
        let params = Params::new().with(1, 3).with(2, 1 << colour_bit).with(3, 1 << car_bit).with(4, 1).with(5, 1);
        let parts = assembler.assemble(155, &params, &mut Repeats::default(), &mut Lcg(seed)).unwrap();
        let header = &index.banks()[parts[0].bank].header;
        assert!(firsts.contains(&header.event), "{}", header.event);
        header.selectors_of(parts[0].take).to_vec()
    };
    // Whatever version and take the dice give, the selectors follow the parameters.
    let mut colours = BTreeSet::new();
    for colour in 0..13u32 {
        let selectors = selectors_of(colour, 5, u64::from(colour));
        assert_eq!(selectors[0], colour as u8, "colour {colour}");
        assert_eq!(selectors[1], 5);
        colours.insert(selectors[0]);
    }
    assert_eq!(colours.len(), 13);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_sentence_is_far_shorter_than_all_its_variants_together() {
    let Some((sentences, index)) = load() else { return };
    let assembler = Assembler::new(&sentences, &index);
    // The vehicle report names 4 phrases per version, but its banks hold hundreds of takes.
    let params = Params::new().with(1, 3).with(2, 1).with(3, 1).with(4, 2).with(5, 1);
    let parts = assembler.assemble(155, &params, &mut Repeats::default(), &mut Lcg(1)).unwrap();
    let takes: usize = sentences.sentence(155).unwrap().blocks[0]
        .entries
        .iter()
        .map(|e| index.banks_for(e.phrase).map(|b| b.header.takes()).sum::<usize>())
        .sum();
    assert!(parts.len() <= 4 && takes > 100, "{} takes of {takes}", parts.len());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_parameters_of_every_recorded_take_find_that_take() {
    let Some((sentences, index)) = load() else { return };
    let assembler = Assembler::new(&sentences, &index);
    let mut rng = Lcg(11);
    let (mut tried, mut failed) = (0, Vec::new());
    for event in sentences.events() {
        let sentence = sentences.sentence(event).unwrap();
        for block in &sentence.blocks {
            for entry in block.entries.iter().filter(|e| e.tests.iter().any(|t| t.param != 0)) {
                for bank in index.banks_for(entry.phrase) {
                    let header = &bank.header;
                    for take in 0..header.takes() {
                        // Parameters that name exactly this take's selectors; the other entries accept anything.
                        let mut params = Params::new();
                        if header.speaker != 0xFFFF {
                            params.set(1, u32::from(header.speaker));
                        }
                        for (test, &bit) in entry.tests.iter().zip(header.selectors_of(take)) {
                            if test.param > 1 || (test.param == 1 && header.speaker == 0xFFFF) {
                                params.set(test.param, 1 << bit);
                            }
                        }
                        tried += 1;
                        let Some(parts) = assembler.assemble(event, &params, &mut Repeats::default(), &mut rng) else {
                            failed.push((event, entry.phrase, take));
                            continue;
                        };
                        // The take said for this phrase (if this version has it) has the selectors asked for.
                        for c in parts.iter().filter(|c| index.banks()[c.bank].header.event == entry.phrase) {
                            let got = index.banks()[c.bank].header.selectors_of(c.take);
                            assert_eq!(got, header.selectors_of(take), "event {event} phrase {}", entry.phrase);
                        }
                    }
                }
            }
        }
    }
    println!(
        "{tried} takes asked for by their own parameters; {} not found: {:?}",
        failed.len(),
        &failed[..failed.len().min(10)]
    );
    assert!(failed.len() * 50 < tried, "{} of {tried}", failed.len());
}
