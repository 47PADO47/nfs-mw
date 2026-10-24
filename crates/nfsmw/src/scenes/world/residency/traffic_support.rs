//! Collision-only support for distant traffic, independent of colour and shadow residency.
use super::{Residency, TileState};
use blackbox_collision::CollisionPack;
use glam::Vec2;
use std::collections::{HashMap, HashSet};

/// The sets are looked up for every wanted section every frame, so they use the fast fixed hasher.
type FastSet<K> = HashSet<K, foldhash::fast::FixedState>;
type FastMap<K, V> = HashMap<K, V, foldhash::fast::FixedState>;

const MAX_EXTRA: usize = 128;
const MAX_PENDING: usize = 2;

/// How far the focus, the spawn point or an actor may move before the wanted sections are worked out again, metres.
const REFRESH_DISTANCE: f32 = 10.0;

#[derive(Default)]
pub(super) struct TrafficSupport {
    /// Whether `wanted` is worked out for the current inputs. Cleared by a change of the inputs or of the zone.
    pub(super) computed: bool,
    focus: Option<(Vec2, f32)>,
    spawn: Vec2,
    actors: Vec<(Vec2, f32)>,
    pub(super) wanted: FastSet<usize>,
    pub(super) resident: FastMap<usize, Vec<u32>>,
    pub(super) pending: FastSet<usize>,
    pub(super) failed: FastSet<usize>,
}

impl TrafficSupport {
    /// Whether new inputs are the ones the wanted sections were worked out for, give or take [`REFRESH_DISTANCE`].
    fn holds_for(&self, focus: Option<(Vec2, f32)>, spawn: Vec2, actors: &[(Vec2, f32)]) -> bool {
        let near = |a: Vec2, b: Vec2| a.distance_squared(b) <= REFRESH_DISTANCE * REFRESH_DISTANCE;
        let same_focus = match (self.focus, focus) {
            (None, None) => true,
            (Some((a, ra)), Some((b, rb))) => ra == rb && near(a, b),
            _ => false,
        };
        same_focus
            && near(self.spawn, spawn)
            && self.actors.len() == actors.len()
            && self.actors.iter().zip(actors).all(|(a, b)| a.1 == b.1 && near(a.0, b.0))
    }
}

impl Residency {
    pub(in crate::scenes::world) fn set_traffic_support(
        &mut self,
        focus: [f32; 2],
        spawn: [f32; 2],
        range: f32,
        actors: Vec<(Vec2, f32)>,
    ) {
        let focus = (range.is_finite() && range > 0.0).then_some((Vec2::from_array(focus), range.min(1600.0)));
        let spawn = Vec2::from_array(spawn);
        let support = &mut self.traffic_support;
        // The inputs are given every frame. Drift under a few metres keeps the sections worked out for the inputs
        // before it, so the comparison is always with the last inputs that were used.
        if support.computed && support.holds_for(focus, spawn, &actors) {
            return;
        }
        support.focus = focus;
        support.spawn = spawn;
        support.actors = actors;
        support.computed = false;
    }

    pub(super) fn update_traffic_support(&mut self) {
        if !self.traffic_support.computed {
            self.traffic_support.computed = true;
            self.refresh_traffic_wanted();
        }
        self.request_traffic_collision();
    }

    /// Works out the sections to hold collision for, and frees the collision of those no longer wanted.
    fn refresh_traffic_wanted(&mut self) {
        let support = &mut self.traffic_support;
        let mut candidates = Vec::new();
        for (index, section) in self.sections.iter().take(self.mod_base).enumerate() {
            if !section.is_spatial() {
                continue;
            }
            let centre = Vec2::from_array(section.center);
            let active = support.actors.iter().any(|(p, r)| centre.distance(*p) <= r + section.radius);
            let distance = support.focus.map_or(f32::INFINITY, |(p, r)| centre.distance(p) - r - section.radius);
            if active || distance <= 0.0 {
                // Keep the bounded cache at the activation band, not filled with the nearest sections
                // behind it. Live actors and their predicted paths still win over every new activation.
                let priority = match active {
                    true => distance,
                    false => section.distance_to(support.spawn.x, support.spawn.y),
                };
                candidates.push((index, !active, priority));
            }
        }
        candidates.sort_unstable_by(|a, b| a.1.cmp(&b.1).then(a.2.total_cmp(&b.2)).then(a.0.cmp(&b.0)));
        support.wanted = candidates.into_iter().take(MAX_EXTRA).map(|c| c.0).collect();
        let mut expired: Vec<_> = support.resident.keys().copied().filter(|i| !support.wanted.contains(i)).collect();
        // The map's order is arbitrary; releases touch the collision, so they go in section order.
        expired.sort_unstable();
        for index in expired {
            self.release_traffic_collision(index);
        }
        // Failed sections can retry after leaving and re-entering their support region.
        let support = &mut self.traffic_support;
        support.failed.retain(|i| support.wanted.contains(i));
    }

    /// Asks for the collision of the wanted sections that are not loaded or on their way.
    fn request_traffic_collision(&mut self) {
        let support = &mut self.traffic_support;
        // Only `MAX_PENDING` loads are in flight at once, so with that many there is nothing to ask for.
        if support.pending.len() >= MAX_PENDING {
            return;
        }
        let mut missing: Vec<_> = support
            .wanted
            .iter()
            .copied()
            .filter(|i| {
                !support.resident.contains_key(i)
                    && !support.pending.contains(i)
                    && !support.failed.contains(i)
                    && !matches!(self.tiles.get(i), Some(TileState::Resident { .. }))
            })
            .collect();
        missing.sort_unstable_by(|a, b| {
            let score = |i: usize| {
                let section = &self.sections[i];
                let centre = Vec2::from_array(section.center);
                let active = support.actors.iter().any(|(p, r)| centre.distance(*p) <= r + section.radius);
                let target = match active {
                    true => support.focus.map_or(support.spawn, |v| v.0),
                    false => support.spawn,
                };
                (!active, section.distance_to(target.x, target.y))
            };
            let (ka, da) = score(*a);
            let (kb, db) = score(*b);
            ka.cmp(&kb).then(da.total_cmp(&db)).then(a.cmp(b))
        });
        for index in missing.into_iter().take(MAX_PENDING.saturating_sub(support.pending.len())) {
            support.pending.insert(index);
            self.streamer.request_collision(index);
        }
    }

    pub(super) fn receive_traffic_collision(&mut self, index: usize, result: Result<Vec<CollisionPack>, String>) {
        self.traffic_support.pending.remove(&index);
        let packs = match result {
            Ok(packs) => packs,
            Err(error) => {
                self.traffic_support.failed.insert(index);
                log::warn!("traffic collision section {index}: {error}");
                return;
            }
        };
        if !self.traffic_support.wanted.contains(&index) {
            return;
        }
        self.traffic_support.resident.insert(index, packs.iter().map(|p| p.section).collect());
        for pack in packs {
            self.collision.insert(pack);
        }
    }

    pub(super) fn retain_traffic_collision(&mut self, index: usize, packs: &[u32]) {
        if self.traffic_support.wanted.contains(&index) {
            self.traffic_support.resident.insert(index, packs.to_vec());
        }
    }

    pub(super) fn traffic_owns_collision(&self, section: u32) -> bool {
        self.traffic_support.resident.values().any(|packs| packs.contains(&section))
    }

    fn release_traffic_collision(&mut self, index: usize) {
        let Some(packs) = self.traffic_support.resident.remove(&index) else { return };
        for section in packs {
            let colour_owner = self
                .tiles
                .values()
                .any(|tile| matches!(tile, TileState::Resident { packs, .. } if packs.contains(&section)));
            if !colour_owner && !self.traffic_owns_collision(section) {
                self.collision.remove(section);
            }
        }
    }
}

#[cfg(test)]
mod tests;
