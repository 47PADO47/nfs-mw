//! One free-roam target's persistent road observation, shared by its pursuit drivers.
//! Spec: `docs/specs/ai-pursuit-reinforcement-driving.md`, shared primary target road.

use blackbox_roads::{Body, RoadNetwork, RoadTracker, SegmentFilter, SegmentIndex};
use glam::Vec2;

use super::{TrafficWorld, cop::TargetRoad};
use crate::scenes::world::StateHasher;

#[derive(Default)]
pub(super) struct PrimaryRoad {
    owner: Option<u8>,
    tracker: RoadTracker,
    observation: Option<TargetRoad>,
}

impl PrimaryRoad {
    #[cfg(test)]
    pub(super) fn observation(&self) -> Option<TargetRoad> {
        self.observation
    }

    fn reset(&mut self) {
        self.tracker = RoadTracker::default();
        self.observation = None;
    }

    pub(super) fn update(
        &mut self,
        net: &RoadNetwork,
        index: &SegmentIndex,
        body: Option<Body>,
        racing: bool,
        jerk: bool,
        dt: f32,
    ) -> Option<TargetRoad> {
        // Racing needs authored route membership, which this free-roam tracker does not consume.
        let body = body.filter(|_| !racing);
        self.tracker.update(net, index, body, dt);
        self.observation = body.and_then(|body| {
            let current = self.tracker.current()?;
            let cop_filter = SegmentFilter { cop: true, ..Default::default() }.allows(net.segment(current.segment));
            let observed = self.tracker.snapshot()?;
            let direction = observed.future - observed.current;
            let direction = Vec2::new(direction.x, direction.z).try_normalize().unwrap_or(body.forward);
            let seek =
                self.tracker.seek_ahead(net, body.velocity.length(), if jerk { 3.12 } else { 7.8 }, cop_filter)?;
            Some(TargetRoad {
                gap: body.position.distance(observed.current),
                direction,
                centre: observed.current,
                seek,
            })
        });
        self.observation
    }

    pub(super) fn hash_into(&self, hash: &mut StateHasher) {
        hash.bool(self.owner.is_some());
        if let Some(owner) = self.owner {
            hash.u32(u32::from(owner));
        }
        self.tracker.visit_state(&mut |word| hash.u32(word));
        hash.bool(self.observation.is_some());
        if let Some(road) = self.observation {
            road.hash_into(hash);
        }
    }
}

impl TrafficWorld {
    /// Select the primary seat before its simulation step; anonymous fixture targets need no seat id.
    pub(in crate::scenes::world) fn note_primary_target(&mut self, owner: u8) {
        if self.target_road.owner != Some(owner) {
            self.target_road.reset();
            self.target_road.owner = Some(owner);
        }
    }

    /// Replacing a vehicle in the primary seat starts a new target road lifetime.
    pub(in crate::scenes::world) fn reset_primary_target_road(&mut self, owner: u8) {
        if self.target_road.owner.is_none_or(|previous| previous == owner) {
            self.target_road.reset();
        }
    }
}
