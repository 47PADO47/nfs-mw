//! The run seed (`--seed`, `NFSMW_SEED`): the seeds of the random sources that single player otherwise takes from the
//! clock. Each source is its own stream, so two sources never draw the same sequence, and none of the multiplayer
//! session rules (`scenes/world/session_rules.rs`) is switched on. Without a seed every function here returns `None`
//! and the caller reads the clock, as before.

use std::sync::atomic::{AtomicU64, Ordering};

/// A random source that the run seed feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// The names of a race's opponents and the player's grid slot (`SessionRules::name_seed`).
    OpponentNames,
    /// Which chase radio line is said.
    ChaseRadio,
    /// Which recording of a police speech event plays.
    SpeechFiles,
    /// The police speech's own choices (thinning, takes).
    SpeechEvents,
    /// The radio's song order.
    RadioPlaylist,
    /// The Quick Race screen's random picks.
    QuickPlay,
    /// The order of the safe house's marker cards.
    Markers,
    /// The first loading tip.
    LoadingTips,
}

const STREAMS: usize = 8;

/// How many generators each stream has handed out, so a second generator of a stream differs from the first.
static DRAWN: [AtomicU64; STREAMS] = [const { AtomicU64::new(0) }; STREAMS];

/// The seed of generator number `index` of `stream` in a run seeded with `seed`: SplitMix64 over the three, so nearby
/// seeds, streams and indices give unrelated values.
pub fn derive(seed: u64, stream: Stream, index: u64) -> u64 {
    let mut z = seed;
    for word in [stream as u64 + 1, index] {
        z = z.wrapping_add(word.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
    }
    z
}

/// The seed of the next generator of `stream`, or `None` when the run has no seed. Generators made in the same order
/// get the same seeds in every run.
pub fn next(stream: Stream) -> Option<u64> {
    let seed = super::options().seed?;
    let index = DRAWN[stream as usize].fetch_add(1, Ordering::Relaxed);
    Some(derive(seed, stream, index))
}

/// The one seed of `stream`, the same on every call, or `None` when the run has no seed.
pub fn fixed(stream: Stream) -> Option<u64> {
    Some(derive(super::options().seed?, stream, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Stream; STREAMS] = [
        Stream::OpponentNames,
        Stream::ChaseRadio,
        Stream::SpeechFiles,
        Stream::SpeechEvents,
        Stream::RadioPlaylist,
        Stream::QuickPlay,
        Stream::Markers,
        Stream::LoadingTips,
    ];

    #[test]
    fn the_same_inputs_give_the_same_seed() {
        assert_eq!(derive(42, Stream::ChaseRadio, 3), derive(42, Stream::ChaseRadio, 3));
    }

    #[test]
    fn streams_indices_and_seeds_all_differ() {
        let mut seen = std::collections::HashSet::new();
        for seed in [0, 1, 42, u64::MAX] {
            for stream in ALL {
                for index in 0..4 {
                    assert!(seen.insert(derive(seed, stream, index)), "{seed} {stream:?} {index}");
                }
            }
        }
    }

    #[test]
    fn every_stream_has_a_counter() {
        assert!(ALL.iter().all(|&stream| (stream as usize) < STREAMS));
        assert_eq!(ALL.iter().map(|&stream| stream as usize).collect::<std::collections::HashSet<_>>().len(), STREAMS);
    }

    #[test]
    fn a_run_without_a_seed_keeps_the_clock() {
        // Tests never set the run's options.
        assert_eq!(next(Stream::RadioPlaylist), None);
        assert_eq!(fixed(Stream::OpponentNames), None);
    }
}
