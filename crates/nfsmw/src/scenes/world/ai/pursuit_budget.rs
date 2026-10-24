//! The chase's cops-to-evade budget, its backup, and the surplus cops it sends away.
//! Spec: `docs/specs/ai-pursuit-cops.md` section 4, `docs/specs/ai-pursuit-heat.md` section 2.1.

use std::collections::HashSet;

use blackbox_pursuit::CooldownStatus;
use blackbox_pursuit::backup::Event;
use blackbox_pursuit::wave::pick_surplus;
use blackbox_roads::RandomSource;

use super::{Focus, TrafficWorld};

/// Metres added to the distance of a cop that cannot see the player when the farthest ones are picked to leave.
const UNSEEN_PENALTY: f32 = 40.0;
/// The most cops one run sends away.
const MOST_PER_RUN: u32 = 2;
/// A cop that sees the player leaves only while more than this many cops do.
const KEEP_WATCHING: u32 = 2;

impl TrafficWorld {
    /// Seconds until the backup arrives, while it was called (`GetBackupETA`).
    pub fn backup_eta(&self) -> Option<f32> {
        self.pursuit.as_ref()?.budget.eta()
    }

    /// The cops still to evade before the wave is over (at least 1).
    #[cfg(test)]
    pub fn cops_to_evade(&self) -> u32 {
        self.pursuit.as_ref().map_or(0, |pursuit| pursuit.budget.remaining())
    }

    /// Follows who is fully engaged and who has left, then runs the backup (`AIPursuit::Update`): a cop that was within
    /// the engagement radius and is gone from the chase (wrecked, left behind, recycled) counts as evaded; the wave shrinks
    /// by that, the backup is called when few remain, and it arrives after its countdown while the player is in sight.
    pub(super) fn update_budget(&mut self, dt: f32, focus: Focus) {
        let Some(player) = focus.body else { return };
        let cooldown = self.police.cooldown_status() == CooldownStatus::CoolDown;
        let in_sight = self.police.is_perp_in_sight();
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        let backup = pursuit.backup();
        let mut now = HashSet::new();
        let mut engaged = 0;
        for car in self.cars.iter_mut().filter(|car| is_chase_cop(car)) {
            if car.physics_position().distance(player.position) < backup.engagement_radius {
                engaged += 1;
                if !cooldown {
                    car.note_engaged();
                }
            }
            if car.was_engaged() {
                now.insert(car.audio_emitter().id);
            }
        }
        let left = pursuit.engaged.difference(&now).count() as u32;
        pursuit.budget.note_evaded(left);
        pursuit.engaged = now;
        if cooldown {
            pursuit.budget.cancel_call();
        }
        if pursuit.budget.update(engaged, !cooldown, backup.call_timer) == Event::Called {
            log::info!(
                "pursuit: {} cops left to evade, the backup is called ({:.0} s)",
                pursuit.budget.remaining(),
                backup.call_timer
            );
        }
        let required = pursuit.row().map_or(0, |row| row.full_engagement_cops);
        if pursuit.budget.tick(dt, in_sight, required, backup.trigger) == Event::Arrived {
            log::info!("pursuit: the backup arrives, {required} cops to evade again");
        }
    }

    /// Sends the cops over what the chase wants away (`RemoveUnwantedVehicles`): of the type the chase has most over, up to
    /// two, the farthest first (a cop that cannot see the player counts as 40 m farther). A cop that sees the player leaves
    /// only while more than two do. A cop that left drives off with its lights out until the patrol pass recycles it.
    pub(super) fn flee_surplus(&mut self, focus: Focus, cooldown: bool) {
        let Some(player) = focus.body else { return };
        let Some(pursuit) = self.pursuit.as_ref() else { return };
        let adjusted: Vec<(String, u32)> = pursuit
            .adjusted_wave(self.police.settings(), cooldown)
            .into_iter()
            .map(|(name, count, _)| (name.to_ascii_lowercase(), count))
            .collect();
        let mut types: Vec<(String, u32)> = Vec::new();
        for car in self.cars.iter().filter(|car| is_chase_cop(car)) {
            let name = car.name.to_ascii_lowercase();
            match types.iter_mut().find(|(known, _)| *known == name) {
                Some((_, count)) => *count += 1,
                None => types.push((name, 1)),
            }
        }
        let surplus: Vec<u32> = types
            .iter()
            .map(|(name, count)| {
                let wanted = adjusted.iter().find(|(known, _)| known == name).map_or(0, |(_, n)| *n);
                count.saturating_sub(wanted)
            })
            .collect();
        if surplus.iter().all(|&over| over == 0) {
            return;
        }
        let roll = self.rng.police.next_f32();
        let Some((index, count)) = pick_surplus(&surplus, roll) else { return };
        let kind = types[index].0.clone();
        let cruise = self.search_cruise();
        let count = count.min(MOST_PER_RUN);
        let Self { cars, network, index: lanes, .. } = self;
        let (mut already, mut watching) = (0u32, 0u32);
        let mut candidates: Vec<(usize, f32, bool)> = Vec::new();
        for (i, car) in cars.iter().enumerate().filter(|(_, car)| is_chase_cop(car)) {
            if car.is_fleeing() {
                already += 1;
                continue;
            }
            let distance = car.physics_position().distance(player.position);
            // Each cop keeps its own answer of the last sight test (`GetTimeSinceTargetSeen() <= 0`).
            let sees = car.sees_player();
            watching += u32::from(sees);
            if car.name.eq_ignore_ascii_case(&kind) {
                candidates.push((i, distance + if sees { 0.0 } else { UNSEEN_PENALTY }, sees));
            }
        }
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (rank, &(i, _, sees)) in candidates.iter().take(MOST_PER_RUN as usize).enumerate() {
            if count <= already + rank as u32 {
                break;
            }
            if sees && watching <= KEEP_WATCHING {
                continue;
            }
            if cars[i].start_flee(cruise, network, lanes) {
                log::info!("pursuit: {} is sent away, over what the chase wants", cars[i].name);
                if sees && rank == 0 {
                    watching -= 1;
                }
            }
        }
    }
}

/// A cop of the chase proper: not a roadblock car and not a support car.
fn is_chase_cop(car: &super::AiCar) -> bool {
    car.is_cop() && !car.is_roadblock() && car.support_serial().is_none()
}
