use ea_audio::speech::{BankHeader, Repeats, SpeechBank};

use crate::audio::speech::lines::{choose_bank, choose_take};
use crate::audio::speech::random::Rng;

fn header(speaker: u16, takes: usize, memory: u8) -> BankHeader {
    BankHeader {
        event: 10,
        speaker,
        starts: (0..takes as u64).map(|t| t * 0x100).collect(),
        memory,
        selector_count: 0,
        selectors: vec![],
        extra: vec![],
    }
}

fn bank(number: u32, speaker: u16, takes: usize) -> SpeechBank {
    SpeechBank { kind: 1, number, base: 0, header: header(speaker, takes, 0) }
}

#[test]
fn the_random_numbers_repeat_for_a_seed_and_stay_in_range() {
    let (mut a, mut b) = (Rng::new(42), Rng::new(42));
    let from_a: Vec<usize> = (0..20).map(|_| a.below(7)).collect();
    assert_eq!(from_a, (0..20).map(|_| b.below(7)).collect::<Vec<_>>());
    assert!(from_a.iter().all(|&n| n < 7));
    assert_eq!(a.below(0), 0);
    for _ in 0..1000 {
        let x = a.next_f32();
        assert!((0.0..1.0).contains(&x));
    }
    // Every value turns up in time.
    let mut seen = [false; 5];
    for _ in 0..200 {
        seen[a.below(5)] = true;
    }
    assert!(seen.iter().all(|&s| s));
}

#[test]
fn a_speaker_says_the_phrase_in_their_own_voice_when_they_recorded_it() {
    let banks = [bank(0, 3, 4), bank(1, 4, 4), bank(2, 0xFFFF, 4)];
    let refs: Vec<&SpeechBank> = banks.iter().collect();
    let mut rng = Rng::new(1);
    for _ in 0..50 {
        let chosen = choose_bank(&refs, 4, &mut rng).unwrap();
        assert!(chosen.header.speaker == 4 || chosen.header.speaker == 0xFFFF);
    }
}

#[test]
fn any_voice_will_do_for_speaker_zero_or_an_unrecorded_speaker() {
    let banks = [bank(0, 3, 4), bank(1, 4, 4)];
    let refs: Vec<&SpeechBank> = banks.iter().collect();
    let mut rng = Rng::new(2);
    let mut numbers = std::collections::BTreeSet::new();
    for speaker in [0, 7] {
        for _ in 0..50 {
            numbers.insert(choose_bank(&refs, speaker, &mut rng).unwrap().number);
        }
    }
    assert_eq!(numbers.into_iter().collect::<Vec<_>>(), [0, 1]);
    assert!(choose_bank(&[], 1, &mut rng).is_none());
}

#[test]
fn a_take_is_not_repeated_while_the_banks_memory_holds_it() {
    let mut rng = Rng::new(5);
    let header = header(3, 6, 4);
    let mut repeats = Repeats::default();
    let mut heard = Vec::new();
    for _ in 0..40 {
        let take = choose_take(&header, 0, &repeats, &mut rng);
        repeats.remember(0, take, usize::from(header.memory));
        heard.push(take);
    }
    for window in heard.windows(5) {
        let mut distinct = window.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 5, "{window:?}");
    }
    assert!(heard.iter().all(|&t| t < 6));
}

#[test]
fn small_banks_and_banks_without_memory_still_give_a_take() {
    let mut rng = Rng::new(9);
    let one = header(1, 1, 1);
    let mut repeats = Repeats::default();
    for _ in 0..3 {
        let take = choose_take(&one, 0, &repeats, &mut rng);
        repeats.remember(0, take, 1);
        assert_eq!(take, 0);
    }
    // Memory as long as the bank: the takes come round in turn, the oldest first once all were heard.
    let two = header(1, 2, 2);
    let mut repeats = Repeats::default();
    let first = choose_take(&two, 1, &repeats, &mut rng);
    repeats.remember(1, first, 2);
    let second = choose_take(&two, 1, &repeats, &mut rng);
    assert_ne!(first, second);
    repeats.remember(1, second, 2);
    assert_eq!(choose_take(&two, 1, &repeats, &mut rng), first, "the one said longest ago");
}
