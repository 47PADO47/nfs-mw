//! Sentences and takes chosen by parameters, on synthetic banks and records.

use crate::speech::{Assembler, Chosen, Options, Params, Random, Repeats, Sentence, Sentences, SpeechIndex, Voice};
use crate::tests::speech::index;

/// A repeatable random source (a linear congruential generator).
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

/// A bank header with the given selectors per take (all takes need as many selectors as the first).
fn bank(event: u16, speaker: u16, selectors: &[&[u8]], memory: u8) -> Vec<u8> {
    let n = selectors.len();
    let per = selectors[0].len();
    let mut out = event.to_le_bytes().to_vec();
    out.extend(speaker.to_le_bytes());
    out.extend([per as u8, n as u8, memory, 0, 0x10, 0, 0, 0, 0, 0]);
    for (t, sel) in selectors.iter().enumerate() {
        out.extend_from_slice(sel);
        if t + 1 < n {
            out.extend(((t as u16 + 1) * 2).to_be_bytes());
        }
    }
    for j in 0..per {
        let union = selectors.iter().fold(0u32, |m, s| m | 1 << s[j]);
        out.extend(union.to_le_bytes());
    }
    out.extend(std::iter::repeat_n(0xFF, usize::from(memory) + 1));
    out
}

type EntryDef = (u16, u8, u8, Vec<(u32, u8)>);
type BlockDef = (u8, u8, Vec<EntryDef>);

/// A record in the layout of the game's file.
fn record(event: u16, blocks: &[BlockDef]) -> Vec<u8> {
    let mut out = event.to_le_bytes().to_vec();
    out.extend([200, 0, 244, 1, blocks.len() as u8, 0, 0, 100, 0, 2]);
    let table = out.len();
    out.extend(std::iter::repeat_n(0u8, 2 * blocks.len().next_multiple_of(2)));
    let mut starts = Vec::new();
    for (weight, chance, entries) in blocks {
        starts.push(out.len() as u16 / 4);
        let n = entries.len();
        let list_words = n.div_ceil(4);
        let mut block = vec![*weight, *chance, (n as u8) << 2, 0, 0, 0, 0, 0];
        let mut at = (2 + list_words) as u8;
        let mut offsets = Vec::new();
        let mut body = Vec::new();
        for (phrase, param, kind, tests) in entries {
            offsets.push(at);
            body.extend(phrase.to_le_bytes());
            body.extend([*param, *kind]);
            body.extend((tests.len() as u32).to_le_bytes());
            for (mask, tag) in tests {
                body.extend(mask.to_le_bytes());
                body.extend([*tag, 0xFF, 0xFF, 0]);
            }
            at += (2 + 2 * tests.len()) as u8;
        }
        offsets.resize(list_words * 4, 0);
        block.extend(offsets);
        block.extend(body);
        out.extend(block);
    }
    for (i, start) in starts.iter().enumerate() {
        out[table + 2 * i..table + 2 * i + 2].copy_from_slice(&start.to_le_bytes());
    }
    out
}

/// A database file holding `records`.
fn evt(records: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0u8; 0x18 + 2 * records.len()];
    out[0x10..0x14].copy_from_slice(&(records.len() as u32).to_le_bytes());
    while out.len() % 4 != 0 {
        out.push(0);
    }
    for (i, r) in records.iter().enumerate() {
        let at = out.len() as u16 / 4;
        out[0x18 + 2 * i..0x18 + 2 * i + 2].copy_from_slice(&at.to_le_bytes());
        out.extend(r);
    }
    out
}

fn index_of(banks: Vec<Vec<u8>>) -> SpeechIndex {
    let list: Vec<(Vec<u8>, u32)> = banks.into_iter().map(|b| (b, 0)).collect();
    SpeechIndex::parse(&index(&list)).unwrap()
}

/// Event 100 says phrase 10 (colour, by speaker), then phrase 11 (anywhere), in the speaker's voice for the first.
/// Parameters: 2 is the colour (bits 0 to 2).
fn colour_world() -> (Sentences, SpeechIndex) {
    let sentence = record(100, &[(0x39, 100, vec![(10, 1, 1, vec![(0b111, 2)]), (11, 0, 0, vec![])])]);
    let banks = vec![
        bank(10, 3, &[&[0], &[1], &[2], &[2]], 0),
        bank(10, 4, &[&[0], &[1], &[2]], 0),
        bank(11, 0xFFFF, &[&[0]], 0),
    ];
    (Sentences::parse(&evt(&[sentence])).unwrap(), index_of(banks))
}

fn take_of(parts: &[Chosen], bank: usize) -> usize {
    parts.iter().find(|c| c.bank == bank).unwrap().take
}

#[test]
fn a_record_of_the_games_layout_parses() {
    // The record of one event of the real file: one block, one entry that tests parameter 2 against 0b11.
    let bytes = [
        0x3D, 0, 0xC8, 0, 0xF4, 1, 1, 0, 0, 0x64, 0, 2, 4, 0, 0, 0, //
        0x39, 0x64, 4, 0, 0, 0, 0, 0, 3, 0, 0, 0, //
        0x2B, 1, 1, 1, 1, 0, 0, 0, 3, 0, 0, 0, 2, 0xFF, 0xFF, 0,
    ];
    let s = Sentence::parse(&bytes).unwrap();
    assert_eq!((s.event, s.blocks.len()), (0x3D, 1));
    let block = &s.blocks[0];
    assert_eq!((block.weight, block.chance), (100, 100));
    let entry = &block.entries[0];
    assert_eq!((entry.phrase, entry.voice), (0x12B, Voice::Speaker(1)));
    assert_eq!((entry.tests[0].mask, entry.tests[0].param), (3, 2));
}

#[test]
fn the_builder_and_the_parser_agree() {
    let r =
        record(7, &[(0x39, 100, vec![(1, 1, 1, vec![(5, 2)]), (2, 0, 0, vec![])]), (0, 50, vec![(3, 0, 0, vec![])])]);
    let s = Sentence::parse(&r).unwrap();
    assert_eq!(s.blocks.len(), 2);
    assert_eq!(s.blocks[0].entries.len(), 2);
    assert_eq!((s.blocks[1].weight, s.blocks[1].chance), (0, 50));
    assert_eq!(s.blocks[0].entries[1].voice, Voice::Any);
}

#[test]
fn the_parameter_picks_the_variant_take() {
    let (sentences, index) = colour_world();
    let assembler = Assembler::new(&sentences, &index);
    for colour in 0..3u8 {
        let params = Params::new().with(1, 3).with(2, 1 << colour);
        let mut rng = Lcg(u64::from(colour));
        let parts = assembler.assemble(100, &params, &mut Repeats::default(), &mut rng).unwrap();
        assert_eq!(parts.len(), 2, "one take per entry, never every variant");
        let take = take_of(&parts, 0);
        assert!(index.banks()[0].header.selectors_of(take) == [colour], "colour {colour}: take {take}");
    }
}

#[test]
fn a_parameter_naming_two_choices_allows_either() {
    let (sentences, index) = colour_world();
    let assembler = Assembler::new(&sentences, &index);
    let params = Params::new().with(1, 3).with(2, 0b101);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..40 {
        let parts = assembler.assemble(100, &params, &mut Repeats::default(), &mut Lcg(seed)).unwrap();
        seen.insert(index.banks()[0].header.selectors_of(take_of(&parts, 0))[0]);
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), [0, 2]);
}

#[test]
fn a_missing_parameter_allows_every_take_and_a_zero_one_none() {
    let (sentences, index) = colour_world();
    let assembler = Assembler::new(&sentences, &index);
    let params = Params::new().with(1, 3);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..60 {
        let parts = assembler.assemble(100, &params, &mut Repeats::default(), &mut Lcg(seed)).unwrap();
        seen.insert(index.banks()[0].header.selectors_of(take_of(&parts, 0))[0]);
    }
    assert_eq!(seen.len(), 3);
    let none = Params::new().with(1, 3).with(2, 0);
    assert!(assembler.assemble(100, &none, &mut Repeats::default(), &mut Lcg(1)).is_none());
}

#[test]
fn the_speaker_fallback_is_a_choice() {
    let (sentences, index) = colour_world();
    let mut assembler = Assembler::new(&sentences, &index);
    let params = Params::new().with(1, 7).with(2, 1);
    let parts = assembler.assemble(100, &params, &mut Repeats::default(), &mut Lcg(3)).unwrap();
    assert!([0, 1].contains(&parts[0].bank), "another voice says it");
    assembler.options = Options { speaker_fallback: false };
    assert!(assembler.assemble(100, &params, &mut Repeats::default(), &mut Lcg(3)).is_none());
    // No speaker asked for: a voice that recorded the phrase says it, in either mode.
    let unasked = Params::new().with(2, 1);
    assert!(assembler.assemble(100, &unasked, &mut Repeats::default(), &mut Lcg(3)).is_some());
}

#[test]
fn takes_said_lately_are_left_out_until_none_is_left() {
    let (sentences, index) = colour_world();
    let assembler = Assembler::new(&sentences, &index);
    // Bank 0 has two takes for colour 2 (takes 2 and 3) and remembers none (memory 0): they may repeat at once.
    let params = Params::new().with(1, 3).with(2, 4);
    let mut rng = Lcg(5);
    let mut seen = std::collections::BTreeSet::new();
    let mut repeats = Repeats::default();
    for _ in 0..30 {
        let parts = assembler.assemble(100, &params, &mut repeats, &mut rng).unwrap();
        seen.insert(take_of(&parts, 0));
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), [2, 3]);

    // With a memory of one take, the same two takes alternate.
    let banks = vec![bank(10, 3, &[&[2], &[2]], 1), bank(11, 0xFFFF, &[&[0]], 0)];
    let index = index_of(banks);
    let assembler = Assembler::new(&sentences, &index);
    let mut repeats = Repeats::default();
    let order: Vec<usize> =
        (0..6).map(|_| take_of(&assembler.assemble(100, &params, &mut repeats, &mut rng).unwrap(), 0)).collect();
    assert!(order.windows(2).all(|w| w[0] != w[1]), "{order:?}");
}

#[test]
fn a_bank_with_only_one_fitting_take_says_it_again() {
    let (sentences, index) = colour_world();
    let assembler = Assembler::new(&sentences, &index);
    let mut repeats = Repeats::default();
    let params = Params::new().with(1, 3).with(2, 1);
    // Bank 1 (speaker 4) has one take for colour 0 and a memory of 0 takes; bank 0 likewise: both repeat.
    let a = assembler.assemble(100, &params, &mut repeats, &mut Lcg(2)).unwrap();
    let b = assembler.assemble(100, &params, &mut repeats, &mut Lcg(2)).unwrap();
    assert_eq!(a, b);
    repeats.clear();
    assert_eq!(repeats.age(0, 0), 0);
}

#[test]
fn versions_are_chosen_by_weight_and_a_failing_version_is_skipped() {
    // Version 0 (weight 3x) needs a take for parameter 2 = 8, which no take of phrase 10 fits; version 1 works.
    let sentence = record(
        100,
        &[(0x39, 100, vec![(10, 1, 1, vec![(0b1000, 2)])]), (0x39, 100, vec![(10, 1, 1, vec![(0b0111, 2)])])],
    );
    let banks = vec![bank(10, 3, &[&[0], &[1], &[2]], 0)];
    let sentences = Sentences::parse(&evt(&[sentence])).unwrap();
    let index = index_of(banks);
    let assembler = Assembler::new(&sentences, &index);
    let params = Params::new().with(1, 3).with(2, 0b1111);
    for seed in 0..20 {
        let parts = assembler.assemble(100, &params, &mut Repeats::default(), &mut Lcg(seed)).unwrap();
        assert_eq!(parts.len(), 1);
    }
    // No version can be said: nothing is.
    let strict = Params::new().with(1, 3).with(2, 0b1000);
    assert!(assembler.assemble(100, &strict, &mut Repeats::default(), &mut Lcg(1)).is_none());
}

#[test]
fn versions_of_weight_zero_come_last_and_chance_gates_a_version() {
    let sentence = record(
        100,
        &[
            (0, 100, vec![(10, 1, 1, vec![])]),
            (0x39, 0, vec![(11, 0, 0, vec![])]),
            (0x39, 100, vec![(12, 0, 0, vec![])]),
        ],
    );
    let banks = vec![bank(10, 3, &[&[]], 0), bank(11, 0xFFFF, &[&[]], 0), bank(12, 0xFFFF, &[&[]], 0)];
    let sentences = Sentences::parse(&evt(&[sentence])).unwrap();
    let index = index_of(banks);
    let assembler = Assembler::new(&sentences, &index);
    for seed in 0..20 {
        let parts =
            assembler.assemble(100, &Params::new().with(1, 3), &mut Repeats::default(), &mut Lcg(seed)).unwrap();
        // Version 1 never plays (chance 0); version 2 is tried before the weight-0 version 0.
        assert_eq!(parts, [Chosen { bank: 2, take: 0 }]);
    }
}

#[test]
fn events_without_a_sentence_say_nothing() {
    let (sentences, index) = colour_world();
    let assembler = Assembler::new(&sentences, &index);
    assert!(assembler.assemble(999, &Params::new(), &mut Repeats::default(), &mut Lcg(1)).is_none());
}

#[test]
fn a_test_on_a_missing_selector_never_fits() {
    // The entry has two tests but the bank has one selector per take: nothing fits (the game would read garbage).
    let sentence = record(100, &[(0x39, 100, vec![(10, 0, 0, vec![(1, 2), (1, 3)])])]);
    let sentences = Sentences::parse(&evt(&[sentence])).unwrap();
    let index = index_of(vec![bank(10, 0xFFFF, &[&[0]], 0)]);
    let assembler = Assembler::new(&sentences, &index);
    assert!(assembler.assemble(100, &Params::new(), &mut Repeats::default(), &mut Lcg(1)).is_none());
}
