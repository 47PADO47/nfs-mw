//! Collision-only support for distant traffic, independent of colour and shadow residency.
use super::{Residency, TileState};
use blackbox_collision::CollisionPack;
use glam::Vec2;
use std::collections::{HashMap, HashSet};

const MAX_EXTRA: usize = 128;
const MAX_PENDING: usize = 2;

#[derive(Default)]
pub(super) struct TrafficSupport {
    focus: Option<(Vec2, f32)>,
    spawn: Vec2,
    actors: Vec<(Vec2, f32)>,
    pub(super) wanted: HashSet<usize>,
    pub(super) resident: HashMap<usize, Vec<u32>>,
    pub(super) pending: HashSet<usize>,
    pub(super) failed: HashSet<usize>,
}

impl Residency {
    pub(in crate::scenes::world) fn set_traffic_support(
        &mut self,
        focus: [f32; 2],
        spawn: [f32; 2],
        range: f32,
        actors: Vec<(Vec2, f32)>,
    ) {
        self.traffic_support.focus =
            (range.is_finite() && range > 0.0).then_some((Vec2::from_array(focus), range.min(1600.0)));
        self.traffic_support.spawn = Vec2::from_array(spawn);
        self.traffic_support.actors = actors;
    }

    pub(super) fn update_traffic_support(&mut self) {
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
        let expired: Vec<_> = support.resident.keys().copied().filter(|i| !support.wanted.contains(i)).collect();
        for index in expired {
            self.release_traffic_collision(index);
        }
        let support = &mut self.traffic_support;
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
        // Failed sections can retry after leaving and re-entering their support region.
        support.failed.retain(|i| support.wanted.contains(i));
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
