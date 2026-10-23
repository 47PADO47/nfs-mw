//! Pursuits: a police car close to a player starts one, a long gap ends it as an evade, and a player left nearly still beside
//! a police car for two seconds is busted and put back on the grid.

use super::ToyWorld;
use super::bots::distance_sq;
use super::player::grid_position;
use crate::SimEvent;

/// A police car this close to a player starts a pursuit.
const START_RANGE: f32 = 80.0;
/// A chased player this far from every police car is getting away.
const EVADE_RANGE: f32 = 200.0;
/// Getting away for this many ticks (five seconds) ends the pursuit as an evade.
const EVADE_TICKS: u16 = 300;
/// A chased player slower than this, within `BUST_RANGE` of a police car, is being busted.
const BUST_SPEED: f32 = 3.0;
const BUST_RANGE: f32 = 6.0;
/// Being busted for this many ticks (two seconds) ends the pursuit as a bust.
const BUST_TICKS: u16 = 120;

impl ToyWorld {
    /// Update every player's pursuit against the police cars, in player-id order.
    pub(super) fn update_chase(&mut self, events: &mut Vec<SimEvent>) {
        if self.police.is_empty() {
            return;
        }
        for (index, slot) in self.players.iter_mut().enumerate() {
            let Some(slot) = slot else {
                continue;
            };
            let nearest_sq =
                self.police.iter().map(|cop| distance_sq(cop.car.pos, slot.car.pos)).fold(f32::INFINITY, f32::min);
            let player = slot.id;
            if !slot.chase.active {
                if nearest_sq < START_RANGE * START_RANGE {
                    slot.chase.active = true;
                    slot.chase.away_ticks = 0;
                    slot.chase.bust_ticks = 0;
                    events.push(SimEvent::PursuitStarted { player });
                }
                continue;
            }
            slot.chase.away_ticks = match nearest_sq > EVADE_RANGE * EVADE_RANGE {
                true => slot.chase.away_ticks.saturating_add(1),
                false => 0,
            };
            if slot.chase.away_ticks >= EVADE_TICKS {
                slot.chase.active = false;
                slot.chase.away_ticks = 0;
                events.push(SimEvent::Evaded { player });
                events.push(SimEvent::PursuitEnded { player });
                continue;
            }
            let held = nearest_sq < BUST_RANGE * BUST_RANGE && slot.car.speed.abs() < BUST_SPEED;
            slot.chase.bust_ticks = match held {
                true => slot.chase.bust_ticks.saturating_add(1),
                false => 0,
            };
            if slot.chase.bust_ticks < BUST_TICKS {
                continue;
            }
            slot.chase.active = false;
            slot.chase.bust_ticks = 0;
            slot.car.pos = grid_position(index);
            slot.car.dir = [0.0, 1.0];
            slot.car.speed = 0.0;
            events.push(SimEvent::Busted { player });
            events.push(SimEvent::PursuitEnded { player });
            events.push(SimEvent::Moved { player });
        }
    }
}
