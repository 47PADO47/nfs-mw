//! The audit of a fitted copy: the checks behind "no tree floats, sinks, stands on a carriageway or hangs low over one", made on the
//! same ground and road lookups [`settle`](super::settle) uses (`docs/formats/chameleon-vegetation.md`, "Standing on the ground"). The
//! real-world run is `audit_real_tests.rs`; the thresholds are the ones the playtest report was written against.

use super::place::{Measure, Placement};
use super::settle::{LEVEL_WINDOW, OVERHANG, ROAD_ABOVE_CROWN, Site, crown_bottom, crown_reach, ground_under};

/// A foot this far off the ground is flagged: floating above it or sunk below it, metres.
pub const FLAG_FOOT: f32 = 0.3;
/// The underside of a crown over a carriageway lower than this is flagged, metres.
pub const FLAG_CROWN: f32 = 3.5;

/// What is wrong with one copy.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Verdict {
    /// The terrain under the foot, when it was found.
    pub ground: Option<f32>,
    /// The foot is more than [`FLAG_FOOT`] above the ground (and no wall or building top holds it up).
    pub floating: bool,
    /// The foot is more than [`FLAG_FOOT`] under the ground.
    pub sunk: bool,
    /// The foot is on a carriageway.
    pub on_road: bool,
    /// The crown reaches more than the allowed overhang over a carriageway with its underside under [`FLAG_CROWN`] over it.
    pub low_crown: bool,
}

impl Verdict {
    /// Whether anything is wrong.
    pub fn any(&self) -> bool {
        self.floating || self.sunk || self.on_road || self.low_crown
    }
}

/// The verdict on the copy `at` of `model` at `site`.
pub fn judge(model: &Measure, at: &Placement, site: &Site) -> Verdict {
    let foot = at.offset;
    let plan = [foot[0], foot[1]];
    let ground = ground_under(site.terrain, foot);
    let held = site.near.supports(plan, foot[2]);
    let mut verdict = Verdict {
        ground,
        floating: ground.is_some_and(|g| foot[2] - g > FLAG_FOOT) && !held,
        sunk: ground.is_some_and(|g| g - foot[2] > FLAG_FOOT),
        ..Verdict::default()
    };
    let Some(roads) = site.roads else { return verdict };
    verdict.on_road = roads.nearest(plan, 0.0, foot[2] - LEVEL_WINDOW, foot[2] + LEVEL_WINDOW).is_some();
    let (reach, bottom) = (crown_reach(model, at), crown_bottom(model, at));
    verdict.low_crown = roads
        .nearest(plan, reach, bottom - FLAG_CROWN, bottom + ROAD_ABOVE_CROWN)
        .is_some_and(|near| reach - near.gap > OVERHANG);
    verdict
}
