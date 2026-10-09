use std::collections::VecDeque;

use ea_audio::speech::{BankHeader, SpeechBank};

use crate::audio::speech::lines::{choose_bank, choose_take};
use crate::audio::speech::random::Rng;

fn bank(number: u32, speaker: u16, takes: usize) -> SpeechBank {
    let header =
        BankHeader { event: 10, speaker, starts: (0..takes as u64).map(|t| t * 0x100).collect(), extra: vec![] };
    SpeechBank { kind: 1, number, base: 0, header }
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
fn a_take_is_not_repeated_until_half_the_bank_has_been_heard() {
    let mut rng = Rng::new(5);
    let mut recent = VecDeque::new();
    let mut heard = Vec::new();
    for _ in 0..40 {
        heard.push(choose_take(6, &mut recent, &mut rng));
    }
    for window in heard.windows(4) {
        let mut distinct = window.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "{window:?}");
    }
    assert!(heard.iter().all(|&t| t < 6));
}

#[test]
fn small_banks_still_give_a_take() {
    let mut rng = Rng::new(9);
    let mut recent = VecDeque::new();
    assert_eq!(choose_take(1, &mut recent, &mut rng), 0);
    assert_eq!(choose_take(1, &mut recent, &mut rng), 0);
    let mut recent = VecDeque::new();
    let first = choose_take(2, &mut recent, &mut rng);
    assert_ne!(choose_take(2, &mut recent, &mut rng), first, "two takes alternate");
}
