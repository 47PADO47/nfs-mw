//! The end of a co-op race (`docs/specs/coop-career.md` section 7). [ours]: the original has no co-op career. The career is
//! credited with the best human place, and once one human has finished the other has `DNF_SECS` to finish too. When that
//! runs out the other is knocked out of the race (the engine's `knock_out`), so the race ends the normal way.

use blackbox_race::{Outcome, RaceEvent, Racer};
use nfsmw_career::coop::credited_place;

use super::RaceSession;
use crate::scenes::world::WorldScene;

/// Seconds the other human has to finish once one human has finished (the DNF timer). [ours]
pub(super) const DNF_SECS: f32 = 30.0;

/// The place a co-op race credits to the career: the best finished human's place, or the last place when no human
/// finished. `humans` holds the outcome of each human racer; `racers` is the number of racers in the race.
pub(super) fn coop_place(humans: &[Outcome], racers: usize) -> u8 {
    let finished: Vec<Option<u8>> = humans.iter().map(finished_place).collect();
    credited_place(&finished).unwrap_or(u8::try_from(racers).unwrap_or(u8::MAX))
}

/// A human's place when it finished, `None` when it did not.
fn finished_place(outcome: &Outcome) -> Option<u8> {
    match outcome.finished {
        true => u8::try_from(outcome.place).ok(),
        false => None,
    }
}

/// Whether the other human's time is up: `DNF_SECS` after the first human finished at `first_finish`, at race time `now`.
pub(super) fn dnf_due(first_finish: Option<f32>, now: f32) -> bool {
    first_finish.is_some_and(|done| now - done >= DNF_SECS)
}

/// The race time at which the first human racer finished, `None` while no human has.
fn first_human_finish(racers: &[Racer]) -> Option<f32> {
    racers.iter().filter(|racer| racer.human).filter_map(|racer| racer.finished_at).reduce(f32::min)
}

impl WorldScene {
    /// The co-op DNF timer, once a tick. When it runs out, every human still racing is knocked out, which ends the race.
    /// Nothing happens outside a co-op race.
    pub(super) fn coop_dnf(&self, session: &mut RaceSession) -> Vec<RaceEvent> {
        if self.coop.is_none() {
            return Vec::new();
        }
        let Some(engine) = session.engine.as_mut() else { return Vec::new() };
        if !dnf_due(first_human_finish(engine.racers()), engine.elapsed()) {
            return Vec::new();
        }
        let stragglers: Vec<usize> = engine
            .racers()
            .iter()
            .enumerate()
            .filter(|(_, racer)| racer.human && racer.racing())
            .map(|(index, _)| index)
            .collect();
        let mut events = Vec::new();
        for index in stragglers {
            events.extend(engine.knock_out(index));
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(racer: usize, place: u32, finished: bool) -> Outcome {
        Outcome { racer, place, time: finished.then_some(60.0), finished }
    }

    #[test]
    fn best_human_place_is_credited() {
        let both = [outcome(0, 3, true), outcome(1, 2, true)];
        assert_eq!(coop_place(&both, 6), 2);
        let one = [outcome(0, 4, true), outcome(1, 6, false)];
        assert_eq!(coop_place(&one, 6), 4);
    }

    #[test]
    fn nobody_finished_counts_as_last() {
        let neither = [outcome(0, 5, false), outcome(1, 6, false)];
        assert_eq!(coop_place(&neither, 6), 6);
    }

    #[test]
    fn dnf_waits_thirty_seconds() {
        assert!(!dnf_due(None, 500.0));
        assert!(!dnf_due(Some(10.0), 39.5));
        assert!(dnf_due(Some(10.0), 40.0));
        assert!(dnf_due(Some(10.0), 41.0));
    }
}
