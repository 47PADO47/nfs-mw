//! Getting a stuck cop going again (spec `ai-pursuit-obstacle-selection.md`, stuck recovery): when the stuck test fires
//! and the reverse starts, the lane cursor is put on the road's centre lane at the car with its path search dropped, so
//! the cop does not drive back into the wall or car it hit; when the reverse ends, the cursor is put at the car again and
//! snapped to the nearest lane it may drive in.

use blackbox_roads::{Body, RoadNav, RoadNetwork, SegmentIndex};
use glam::Vec3;

use super::CopState;

impl CopState {
    /// One think of the recovery: `stuck` when the stuck test fired this think, `reversing` while the reverse runs.
    pub(in crate::scenes::world::ai) fn recover(
        &mut self,
        net: &RoadNetwork,
        index: &SegmentIndex,
        nav: &mut RoadNav,
        body: &Body,
        stuck: bool,
        reversing: bool,
    ) {
        let at = body.position;
        let forward = Vec3::new(body.forward.x, 0.0, body.forward.y);
        if stuck {
            self.recovering = true;
            self.path_state = None;
            if nav.init_at_point(net, index, at, forward, true) {
                nav.enable_trail(net);
            }
            return;
        }
        if !self.recovering || reversing {
            return;
        }
        self.recovering = false;
        if !nav.init_at_point(net, index, at, forward, false) {
            return;
        }
        // No selectable lane here leaves the cursor where the initialisation put it.
        let _ = nav.snap_offset_at(net, at);
        nav.enable_trail(net);
    }
}
