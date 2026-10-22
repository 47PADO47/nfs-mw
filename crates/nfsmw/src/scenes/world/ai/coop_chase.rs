//! The police's side of a co-op chase (`docs/specs/coop-career.md` section 5, ADR 0010): one pursuit, one heat and one
//! cool-down, with several target cars. The police keep chasing one car, the primary, through the single-player rules;
//! the world tells them about the other chased cars here. Cops are shared out between all the targets
//! (`blackbox_pursuit::targets::assign`), a secondary target seen by a cop keeps the cool-down from running, and while
//! two or more targets are chased each target's bust is the world's own meter, so the police's meter holds.

use blackbox_pursuit::busted::{BustInput, BustTuning};
use blackbox_pursuit::targets::{TargetId, assign};
use blackbox_roads::Body;
use glam::Vec3;

use super::{PlayerView, TrafficWorld};
use crate::scenes::world::space;

/// How near (metres, horizontally) a cop must be to see a secondary target: the rewrite's stand-in for the sight test
/// it runs for the primary [ours].
pub const SECONDARY_SIGHT: f32 = 60.0;

impl TrafficWorld {
    /// The secondary targets of the chase this tick: each chased car after the primary, with its body. Empty outside
    /// co-op, which leaves every cop on the primary.
    pub fn set_secondary_targets(&mut self, targets: impl IntoIterator<Item = (TargetId, Body)>) {
        self.secondary_targets.clear();
        self.secondary_targets.extend(targets);
    }

    /// A secondary target is seen this tick: the cool-down waits as it does while the primary is seen.
    pub fn set_secondary_seen(&mut self, seen: bool) {
        self.secondary_seen = seen;
    }

    /// Holds the police's own busted meter while the world runs one meter per target (two or more chased).
    pub fn hold_shared_bust(&mut self, hold: bool) {
        self.shared_bust_held = hold;
    }

    /// The horizontal distance (metres) from `at` (render space) to the nearest height-matched cop; infinite with none.
    pub fn cop_distance(&self, at: Vec3) -> f32 {
        let at = Vec3::from_array(space::to_physics(at));
        self.nearest_matched_cop_position(at).map_or(f32::INFINITY, |cop| {
            let gap = cop - at;
            gap.x.hypot(gap.z)
        })
    }

    /// The busted timer's constants.
    pub fn bust_tuning(&self) -> BustTuning {
        self.police.bust_tuning()
    }

    /// What the busted timer of one target reads this tick. The primary is seen by the police's own sight test; a
    /// secondary target counts as seen with a cop within [`SECONDARY_SIGHT`].
    pub fn bust_input_for(&self, view: &PlayerView, primary: bool) -> BustInput {
        let level = self.police.level().unwrap_or(1);
        let cop_distance = self.cop_distance(view.position);
        let seen = match primary {
            true => self.police.is_perp_in_sight(),
            false => cop_distance < SECONDARY_SIGHT,
        };
        BustInput {
            cop_distance,
            seen,
            player_speed: view.speed.abs(),
            slow_speed: self.police.bust_speed(level),
            flashing: false,
        }
    }

    /// Each cop's own target this step when the chase has secondary targets: `Some` for a cop sent after a secondary
    /// car, `None` for one that stays on the primary (and for every car that is not a chasing cop). Empty without
    /// secondary targets, so a single-player chase does no work here.
    pub(super) fn secondary_assignments(&self, primary: Option<Body>) -> Vec<Option<Body>> {
        let Some(primary) = primary.filter(|_| !self.secondary_targets.is_empty()) else { return Vec::new() };
        let chasing: Vec<usize> = (0..self.cars.len())
            .filter(|&i| self.cars[i].is_cop() && self.cars[i].support_serial().is_none())
            .collect();
        let cops: Vec<Vec3> = chasing.iter().map(|&i| self.cars[i].avoidable().position).collect();
        let mut targets = vec![(PRIMARY, primary.position)];
        targets.extend(self.secondary_targets.iter().map(|(id, body)| (*id, body.position)));
        let picks = assign(&cops, &targets);
        let mut out = vec![None; self.cars.len()];
        for (&car, pick) in chasing.iter().zip(picks) {
            out[car] = pick.filter(|&id| id != PRIMARY).and_then(|id| self.secondary_body(id));
        }
        out
    }

    fn secondary_body(&self, id: TargetId) -> Option<Body> {
        self.secondary_targets.iter().find(|(target, _)| *target == id).map(|(_, body)| *body)
    }
}

/// The primary's id in the assignment: no secondary target uses it (the world numbers them from its player ids).
const PRIMARY: TargetId = TargetId(u8::MAX);
