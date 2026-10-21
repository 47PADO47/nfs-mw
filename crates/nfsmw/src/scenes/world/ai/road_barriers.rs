//! The track barriers on the road graph: the segments that cross a switched-on barrier line are flagged, so the GPS
//! routes around closed roads (`docs/specs/race-barriers.md` section 5, `docs/specs/ai-road-network.md` section 5.3).
//! The scene says which lines are on (`WorldScene::refresh_road_barriers`).

use blackbox_roads::{ActiveBarrier, SegmentFilter, TrackBarrier, closest_segment, mark_barrier_crossings};
use glam::Vec3;

use super::TrafficWorld;

/// The exemption point outside a race: the road there keeps no barrier flag. Physics position and direction.
const FREE_ROAM_EXEMPTION: (Vec3, Vec3) = (Vec3::new(-2511.0, 147.8, 1783.0), Vec3::new(-0.7, 0.0, 0.7));

impl TrafficWorld {
    /// The track's barrier lines (`0x3414D`), all off.
    pub fn set_track_barriers(&mut self, lines: Vec<TrackBarrier>) {
        self.track_barriers = lines;
    }

    /// The track's barrier lines.
    pub fn track_barriers(&self) -> &[TrackBarrier] {
        &self.track_barriers
    }

    /// Clears the barrier flags of every segment and sets them again for `active` lines, sparing the road at the
    /// free-roam exemption point. Returns how many segments carry a flag.
    pub fn mark_road_barriers(&mut self, active: &[ActiveBarrier]) -> usize {
        let (at, heading) = FREE_ROAM_EXEMPTION;
        let exempt: Vec<u16> = closest_segment(&self.network, &self.index, at, heading, 1.0, SegmentFilter::default())
            .and_then(|located| self.network.segment(located.segment).road)
            .into_iter()
            .collect();
        mark_barrier_crossings(&mut self.network, &self.index, active, &exempt)
    }
}
