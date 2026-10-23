//! Circuit racing: checkpoints in order, laps, and the finishing order.

use super::ToyWorld;
use super::bots::distance_sq;
use super::player::ticks_to_ms;
use crate::SimEvent;

impl ToyWorld {
    /// Advance every unfinished player's circuit progress by at most one checkpoint. The lap event comes before the finish
    /// event; players who finish on the same tick are placed in id order.
    pub(super) fn update_race(&mut self, events: &mut Vec<SimEvent>) {
        let Some(race) = &self.config.race else {
            return;
        };
        let now = self.tick.wrapping_add(1);
        let radius_sq = race.radius * race.radius;
        for slot in self.players.iter_mut().flatten() {
            if slot.race.finished {
                continue;
            }
            let target = race.checkpoints[usize::from(slot.race.next)];
            if distance_sq(slot.car.pos, target) > radius_sq {
                continue;
            }
            slot.race.next += 1;
            if usize::from(slot.race.next) < race.checkpoints.len() {
                continue;
            }
            slot.race.next = 0;
            slot.race.laps = slot.race.laps.saturating_add(1);
            let lap_ms = ticks_to_ms(now.wrapping_sub(slot.race.lap_start));
            events.push(SimEvent::LapCompleted { player: slot.id, lap: slot.race.laps, time_ms: lap_ms });
            slot.race.lap_start = now;
            if slot.race.laps < race.laps {
                continue;
            }
            slot.race.finished = true;
            self.finished = self.finished.saturating_add(1);
            let time_ms = ticks_to_ms(now.wrapping_sub(slot.race.start));
            events.push(SimEvent::RaceFinished { player: slot.id, place: self.finished, time_ms });
        }
    }
}
