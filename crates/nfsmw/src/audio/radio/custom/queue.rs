//! The order in which one list's custom tracks play: in list order, or shuffled one round at a time.
//!
//! A round is every track once. In shuffle mode a new round is a random order, and its first track is not the last
//! track of the round before when the list has more than one track.

use crate::audio::radio::Mode;

/// The order state of one list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Queue {
    len: usize,
    /// The positions of the current round, in the order they play.
    order: Vec<usize>,
    /// How many of `order` have played in this round. A round is over when this reaches `len`.
    played: usize,
    /// The position picked last.
    last: Option<usize>,
}

impl Queue {
    /// A queue over `len` tracks. The first call starts a round.
    pub fn new(len: usize) -> Self {
        Self { len, order: Vec::new(), played: len, last: None }
    }

    /// The next position (0 to `len - 1`), or `None` for an empty list. `random(count)` returns a number below `count`.
    pub fn next(&mut self, mode: Mode, random: &mut dyn FnMut(usize) -> usize) -> Option<usize> {
        if self.len == 0 {
            return None;
        }
        if self.played >= self.len {
            self.start_round(mode, random);
        }
        let pick = *self.order.get(self.played)?;
        self.played += 1;
        self.last = Some(pick);
        Some(pick)
    }

    fn start_round(&mut self, mode: Mode, random: &mut dyn FnMut(usize) -> usize) {
        self.order = (0..self.len).collect();
        self.played = 0;
        if mode == Mode::Shuffle {
            // Fisher-Yates: each position takes a track from those before it (or itself).
            for i in (1..self.len).rev() {
                let j = random(i + 1).min(i);
                self.order.swap(i, j);
            }
            if self.len > 1 && self.last == self.order.first().copied() {
                self.order.swap(0, 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A random source that always answers `count - 1`: the shuffle keeps the identity order.
    fn keep_order(count: usize) -> usize {
        count - 1
    }

    fn take(queue: &mut Queue, mode: Mode, random: fn(usize) -> usize, n: usize) -> Vec<usize> {
        (0..n).map(|_| queue.next(mode, &mut |count| random(count)).unwrap()).collect()
    }

    #[test]
    fn in_list_order_the_list_wraps_to_its_start() {
        let mut queue = Queue::new(3);
        assert_eq!(take(&mut queue, Mode::Ordered, keep_order, 7), [0, 1, 2, 0, 1, 2, 0]);
    }

    #[test]
    fn an_empty_list_gives_nothing_and_a_single_track_repeats() {
        assert_eq!(Queue::new(0).next(Mode::Shuffle, &mut keep_order), None);
        let mut one = Queue::new(1);
        assert_eq!(take(&mut one, Mode::Shuffle, keep_order, 3), [0, 0, 0]);
    }

    #[test]
    fn a_shuffled_round_holds_every_track_once_before_any_repeats() {
        // A small generator, so the orders differ from round to round.
        let mut state = 7u64;
        let mut queue = Queue::new(5);
        for _ in 0..4 {
            let mut round: Vec<usize> = (0..5)
                .map(|_| {
                    queue
                        .next(Mode::Shuffle, &mut |count| {
                            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                            (state >> 33) as usize % count
                        })
                        .unwrap()
                })
                .collect();
            round.sort_unstable();
            assert_eq!(round, [0, 1, 2, 3, 4]);
        }
    }

    #[test]
    fn a_new_round_does_not_start_with_the_track_that_ended_the_last() {
        let mut queue = Queue::new(3);
        // The first round is in order (0, 1, 2), so the next round would start with 2 if the random source kept it.
        assert_eq!(take(&mut queue, Mode::Shuffle, keep_order, 3), [0, 1, 2]);
        queue.last = Some(0);
        queue.order = vec![0, 1, 2];
        queue.played = 3;
        let first = queue.next(Mode::Shuffle, &mut keep_order).unwrap();
        assert_ne!(first, 0);
    }
}
