//! Shared formation members, approach offsets and readiness. Spec: `ai-pursuit-formations.md` sections 3 and 8.

use blackbox_pursuit::formation::{Formation, assignment, slots};
use blackbox_roads::{SegmentFilter, centre_line, closest_segment, right_of, travel_profile, zone};
use glam::{Vec2, Vec3};

use super::super::TrafficWorld;

/// Stable entity-to-slot bindings survive movement during a finisher.
#[derive(Default)]
pub(in crate::scenes::world::ai) struct Membership {
    formation: Option<Formation>,
    slots: Vec<(u32, usize)>,
    offsets: Vec<(u32, Vec2)>,
    pub(super) collapsing: bool,
}

pub(in crate::scenes::world::ai) struct Layout {
    pub cops: Vec<usize>,
    pub positions: Vec<Vec2>,
    pub eligible: Vec<bool>,
    pub slots: Vec<Option<usize>>,
    pub aims: Vec<Vec2>,
}

impl TrafficWorld {
    /// The actual current formation offset supplied to this cop, for speech and presentation.
    pub(in crate::scenes::world::ai) fn formation_slot(&self, key: u32) -> Option<Vec2> {
        self.police.last_formation()?;
        self.formation_membership.borrow().offsets.iter().find(|(id, _)| *id == key).map(|(_, offset)| *offset)
    }

    /// A collapse with real eligible members was selected by the latest physics step.
    pub fn collapse_active(&self) -> bool {
        self.is_pursuing()
            && self.police.is_perp_in_sight()
            && !self.police.is_busted()
            && self.police.last_formation().is_some()
            && !self.police.finisher_running()
            && self.formation_membership.borrow().collapsing
    }

    pub(in crate::scenes::world::ai) fn formation_gap_input(
        &self,
        target: Vec3,
        forward: Vec3,
        velocity: Vec3,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) -> Option<(u32, f32)> {
        self.police.last_formation()?;
        let forward = Vec3::from_array(crate::scenes::world::space::to_physics(forward));
        let velocity = Vec3::from_array(crate::scenes::world::space::to_physics(velocity));
        let direction = blackbox_pursuit::goals::approach::motion_vector(
            Vec2::new(velocity.x, velocity.z),
            Vec2::new(forward.x, forward.z),
        )
        .normalize_or_zero();
        // The gap reads only the slotted cops: the grid for the cars without a slot changes their aims alone.
        let layout = self.formation_layout_with(target, direction, clear, false);
        let gaps: Vec<f32> = layout
            .positions
            .iter()
            .zip(&layout.aims)
            .zip(&layout.slots)
            .filter_map(|((p, aim), slot)| slot.map(|_| p.distance(*aim)))
            .filter(|&gap| gap < 4.0)
            .collect();
        if gaps.is_empty() {
            return None;
        }
        Some((gaps.len() as u32, gaps.iter().sum::<f32>() / gaps.len() as f32))
    }

    pub(in crate::scenes::world::ai) fn formation_layout(
        &self,
        target: Vec3,
        forward: Vec2,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) -> Layout {
        self.formation_layout_with(target, forward, clear, true)
    }

    /// The layout of the cops around `target`. `extras` places the cops without a slot on the grid behind the
    /// formation (their aims); the slots, the membership and every slotted cop's aim are the same without it.
    fn formation_layout_with(
        &self,
        target: Vec3,
        forward: Vec2,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
        extras: bool,
    ) -> Layout {
        let forward = forward.try_normalize().unwrap_or(Vec2::Y);
        let right = Vec2::new(forward.y, -forward.x);
        let player = Vec2::new(target.x, target.z);
        let heading = Vec3::new(forward.x, 0.0, forward.y);
        let cops: Vec<usize> = (0..self.cars.len())
            .filter(|&i| {
                let car = &self.cars[i];
                car.is_cop() && !car.is_fleeing() && car.support_serial().is_none()
            })
            .collect();
        // The road sets the sight range and the herd's lanes, and both only matter to a cop.
        let road = match cops.is_empty() {
            true => None,
            false => closest_segment(&self.network, &self.index, target, heading, 0.0, SegmentFilter::default()),
        };
        let range = 60.0 + road.map_or(0.0, |hit| hit.point.distance(target));
        let positions: Vec<Vec2> = cops
            .iter()
            .map(|&i| {
                let p = self.cars[i].avoidable().position;
                Vec2::new(p.x, p.z)
            })
            .collect();
        let local: Vec<Vec2> = positions
            .iter()
            .map(|p| {
                let rel = *p - player;
                Vec2::new(rel.dot(right), rel.dot(forward))
            })
            .collect();
        let eligible: Vec<bool> = cops
            .iter()
            .map(|&i| {
                let position = self.cars[i].avoidable().position;
                position.distance(target) < range && clear(position + Vec3::Y, target + Vec3::Y)
            })
            .collect();
        let mut slots_of = vec![None; cops.len()];
        let mut aims = vec![player; cops.len()];
        if let Some(formation) = self.police.last_formation() {
            let tuning = self.police.goal_tuning();
            let tightness = match formation {
                Formation::RollingBlock => tuning.rolling_block_tightness,
                _ => tuning.boxin_tightness,
            };
            let mut offsets = slots::offsets(formation, self.police.formation_progress(), tightness);
            if formation == Formation::Herd
                && let Some(hit) = road
            {
                let curve = centre_line(&self.network, hit.segment, 1);
                let stored = curve.tangent(hit.t).normalize_or_zero();
                let direction = match stored.dot(heading) >= 0.0 {
                    true => 1,
                    false => 0,
                };
                let travel = match direction {
                    1 => stored,
                    _ => -stored,
                };
                let extent = |heading_end| {
                    let profile = travel_profile(&self.network, hit.segment, direction, heading_end);
                    profile.lanes_of(zone::TRAFFIC, true).iter().map(|&i| profile.signed_offset(i)).reduce(f32::max)
                };
                if let (Some(start), Some(end)) = (extent(false), extent(true)) {
                    let along = match direction {
                        1 => hit.t,
                        _ => 1.0 - hit.t,
                    };
                    let outer = start + (end - start) * along;
                    let lateral = (target - hit.point).dot(right_of(travel));
                    offsets = slots::herd_offsets(slots::herd_crowd(lateral, outer));
                }
            }
            let mut membership = self.formation_membership.borrow_mut();
            match self.police.finisher_running() && membership.formation == Some(formation) {
                true => {
                    for (i, &car) in cops.iter().enumerate() {
                        if eligible[i] {
                            slots_of[i] = membership
                                .slots
                                .iter()
                                .find(|(key, _)| *key == self.cars[car].key())
                                .map(|(_, slot)| *slot);
                        }
                    }
                }
                false => {
                    let candidates: Vec<usize> = (0..cops.len()).filter(|&i| eligible[i]).collect();
                    let points: Vec<Vec2> = candidates.iter().map(|&i| local[i]).collect();
                    for (&cop, slot) in candidates.iter().zip(assignment::assign(formation, &points, &offsets)) {
                        slots_of[cop] = slot;
                    }
                    membership.formation = Some(formation);
                    membership.slots = cops
                        .iter()
                        .zip(&slots_of)
                        .filter_map(|(&car, slot)| slot.map(|s| (self.cars[car].key(), s)))
                        .collect();
                }
            }
            for (cop, slot) in slots_of.iter().enumerate() {
                if let Some(slot) = slot {
                    aims[cop] = player + right * offsets[*slot].x + forward * offsets[*slot].y;
                }
            }
            membership.offsets = cops
                .iter()
                .zip(&slots_of)
                .filter_map(|(&car, slot)| slot.map(|s| (self.cars[car].key(), offsets[s])))
                .collect();
        }
        if !extras || self.police.last_formation().is_none() {
            return Layout { cops, positions, eligible, slots: slots_of, aims };
        }
        let extra: Vec<usize> = (0..cops.len()).filter(|&i| slots_of[i].is_none()).collect();
        let points: Vec<Vec2> = extra.iter().map(|&i| local[i]).collect();
        let grid = assignment::grid(extra.len());
        for (&cop, slot) in extra.iter().zip(assignment::match_offsets(&points, &grid)) {
            if let Some(slot) = slot {
                aims[cop] = player + right * grid[slot].x + forward * grid[slot].y;
            }
        }
        Layout { cops, positions, eligible, slots: slots_of, aims }
    }
}

impl Membership {
    pub(in crate::scenes::world::ai) fn hash_into(&self, hash: &mut crate::scenes::world::StateHasher) {
        hash.bytes(format!("{:?}", self.formation).as_bytes());
        hash.bool(self.collapsing);
        hash.u64(self.slots.len() as u64);
        for &(key, slot) in &self.slots {
            hash.u32(key);
            hash.u64(slot as u64);
        }
        for &(key, offset) in &self.offsets {
            hash.u32(key);
            hash.f32(offset.x);
            hash.f32(offset.y);
        }
    }
}
