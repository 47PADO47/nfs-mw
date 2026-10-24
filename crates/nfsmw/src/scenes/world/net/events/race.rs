//! Race events per player from the race engine's racers after each tick (`blackbox_race::Racer`): a new entry in a
//! racer's lap times is a lap, a finish time is the finish. Places count every racer, the computer's too (which finish
//! without an event); players that finish in the same tick are placed after the computer racers of that tick, in racer
//! order.

use nfsmw_sim::{PlayerId, SimEvent};

/// One racer of the running race, as the events need it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RacerState {
    /// The player in the seat, `None` for a computer racer.
    pub player: Option<PlayerId>,
    /// The race time at the end of each lap, seconds.
    pub lap_times: Vec<f32>,
    /// The race time of the finish, seconds.
    pub finished_at: Option<f32>,
    /// The stored speed of each speed trap in m/s, by slot; `None` for a trap not passed.
    pub trap_speeds: Vec<Option<f32>>,
}

/// What the players' racers had done at the last tick.
#[derive(Debug, Clone, Default)]
pub struct RaceTracker {
    /// Per player: laps done and whether finished.
    seen: Vec<(PlayerId, usize, bool)>,
    /// Per player: the stored speed of each speed trap, as of the last tick.
    traps: Vec<(PlayerId, Vec<Option<f32>>)>,
}

/// Metres per second to kilometres per hour.
const MS_TO_KMH: f32 = 3.6;

/// Seconds to whole milliseconds, 0 for a time that is not a finite positive number.
fn ms(seconds: f32) -> u32 {
    match seconds.is_finite() && seconds > 0.0 {
        true => (f64::from(seconds) * 1000.0).round().min(f64::from(u32::MAX)) as u32,
        false => 0,
    }
}

impl RaceTracker {
    /// Compare the racers after this tick (`None` with no race running) with the last tick's; the events go to `events`.
    pub fn update(&mut self, racers: Option<&[RacerState]>, events: &mut Vec<SimEvent>) {
        let Some(racers) = racers else {
            self.seen.clear();
            self.traps.clear();
            return;
        };
        let mut newly_finished = Vec::new();
        for (index, racer) in racers.iter().enumerate() {
            let Some(player) = racer.player else { continue };
            let (laps, finished) = self.seen_of(player);
            for lap in laps..racer.lap_times.len() {
                let start = lap.checked_sub(1).and_then(|before| racer.lap_times.get(before)).copied().unwrap_or(0.0);
                let time_ms = ms(racer.lap_times[lap] - start);
                let lap = u8::try_from(lap + 1).unwrap_or(u8::MAX);
                events.push(SimEvent::LapCompleted { player, lap, time_ms });
            }
            self.set_seen(player, racer.lap_times.len(), finished || racer.finished_at.is_some());
            self.trap_passes(player, &racer.trap_speeds, events);
            if !finished && racer.finished_at.is_some() {
                newly_finished.push(index);
            }
        }
        let finished_before = racers.iter().filter(|racer| racer.finished_at.is_some()).count() - newly_finished.len();
        for (rank, index) in newly_finished.into_iter().enumerate() {
            let racer = &racers[index];
            let (Some(player), Some(at)) = (racer.player, racer.finished_at) else { continue };
            let place = u8::try_from(finished_before + rank + 1).unwrap_or(u8::MAX);
            events.push(SimEvent::RaceFinished { player, place, time_ms: ms(at) });
        }
    }

    /// A trap whose stored speed is new or higher than last tick's was passed (a pass pays only when it beats the best).
    fn trap_passes(&mut self, player: PlayerId, speeds: &[Option<f32>], events: &mut Vec<SimEvent>) {
        let at = match self.traps.iter().position(|(id, _)| *id == player) {
            Some(at) => at,
            None => {
                self.traps.push((player, Vec::new()));
                self.traps.len() - 1
            }
        };
        let before = &mut self.traps[at].1;
        for (slot, speed) in speeds.iter().enumerate() {
            let Some(speed) = *speed else { continue };
            if before.get(slot).copied().flatten() == Some(speed) {
                continue;
            }
            let trap = u8::try_from(slot).unwrap_or(u8::MAX);
            events.push(SimEvent::SpeedTrapPassed { player, trap, kmh: speed * MS_TO_KMH });
        }
        *before = speeds.to_vec();
    }

    fn seen_of(&self, player: PlayerId) -> (usize, bool) {
        self.seen.iter().find(|(id, _, _)| *id == player).map_or((0, false), |&(_, laps, done)| (laps, done))
    }

    fn set_seen(&mut self, player: PlayerId, laps: usize, finished: bool) {
        match self.seen.iter_mut().find(|(id, _, _)| *id == player) {
            Some(entry) => *entry = (player, laps.max(entry.1), finished),
            None => self.seen.push((player, laps, finished)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: PlayerId = PlayerId(0);
    const TWO: PlayerId = PlayerId(1);

    fn racer(player: Option<PlayerId>, laps: &[f32], finished_at: Option<f32>) -> RacerState {
        RacerState { player, lap_times: laps.to_vec(), finished_at, trap_speeds: Vec::new() }
    }

    fn run(tracker: &mut RaceTracker, racers: Option<&[RacerState]>) -> Vec<SimEvent> {
        let mut events = Vec::new();
        tracker.update(racers, &mut events);
        events
    }

    #[test]
    fn laps_come_once_with_their_own_time() {
        let mut tracker = RaceTracker::default();
        assert_eq!(run(&mut tracker, Some(&[racer(Some(ONE), &[], None)])), []);
        let lap = [racer(Some(ONE), &[61.5], None)];
        assert_eq!(run(&mut tracker, Some(&lap)), [SimEvent::LapCompleted { player: ONE, lap: 1, time_ms: 61_500 }]);
        assert_eq!(run(&mut tracker, Some(&lap)), []);
        let second = [racer(Some(ONE), &[61.5, 120.0], None)];
        assert_eq!(run(&mut tracker, Some(&second)), [SimEvent::LapCompleted { player: ONE, lap: 2, time_ms: 58_500 }]);
    }

    #[test]
    fn a_trap_pass_comes_once_and_again_only_when_it_beats_the_best() {
        let mut tracker = RaceTracker::default();
        let at = |speeds: &[Option<f32>]| {
            [RacerState { player: Some(ONE), lap_times: Vec::new(), finished_at: None, trap_speeds: speeds.to_vec() }]
        };
        assert_eq!(run(&mut tracker, Some(&at(&[None, None]))), []);
        let first = at(&[None, Some(50.0)]);
        assert_eq!(run(&mut tracker, Some(&first)), [SimEvent::SpeedTrapPassed { player: ONE, trap: 1, kmh: 180.0 }]);
        assert_eq!(run(&mut tracker, Some(&first)), [], "a pass comes once");
        let better = at(&[Some(40.0), Some(55.0)]);
        assert_eq!(
            run(&mut tracker, Some(&better)),
            [
                SimEvent::SpeedTrapPassed { player: ONE, trap: 0, kmh: 144.0 },
                SimEvent::SpeedTrapPassed { player: ONE, trap: 1, kmh: 198.0 },
            ]
        );
        assert_eq!(run(&mut tracker, None), []);
        assert_eq!(run(&mut tracker, Some(&first)).len(), 1, "a new race starts clean");
    }

    #[test]
    fn places_count_the_computer_racers_and_same_tick_finishes_go_in_racer_order() {
        let mut tracker = RaceTracker::default();
        let start = [racer(Some(ONE), &[], None), racer(None, &[], None), racer(Some(TWO), &[], None)];
        run(&mut tracker, Some(&start));
        let rival = [racer(Some(ONE), &[], None), racer(None, &[90.0], Some(90.0)), racer(Some(TWO), &[], None)];
        assert_eq!(run(&mut tracker, Some(&rival)), [], "a computer racer finishes without an event");
        let both =
            [racer(Some(ONE), &[], Some(95.25)), racer(None, &[90.0], Some(90.0)), racer(Some(TWO), &[], Some(95.25))];
        assert_eq!(
            run(&mut tracker, Some(&both)),
            [
                SimEvent::RaceFinished { player: ONE, place: 2, time_ms: 95_250 },
                SimEvent::RaceFinished { player: TWO, place: 3, time_ms: 95_250 },
            ]
        );
        assert_eq!(run(&mut tracker, Some(&both)), [], "a finish comes once");
        assert_eq!(run(&mut tracker, None), []);
        assert_eq!(run(&mut tracker, Some(&start)), [], "a new race starts clean");
    }
}
