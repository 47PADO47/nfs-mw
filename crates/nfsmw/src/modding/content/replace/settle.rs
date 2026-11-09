//! Settling the copies of a stretch of trees on the real ground and off the road (`[fit] settle`, `docs/modding/api-replace.md`,
//! "Settling"; findings and numbers in `docs/formats/chameleon-vegetation.md`, "Standing on the ground").
//!
//! [`place`](super::place) puts each copy of a world-space stretch on **the lowest vertex of the old cell it replaces**. That is the ground
//! only when the old geometry reached it. The 2005 tree lines are crowns with no trunk, whose lowest leaves hang metres above the ground
//! (over a road they are the canopy the cars drive under), so a 2012 tree stood on that floor has its trunk ending in the air; and the
//! lowest vertex of a cell on a slope is under the ground at the cell's middle, so a tree there sinks. Two more things the placement
//! cannot see: the middle of a cell may be on the carriageway (the old crown reached over the road from a trunk beside it), and a
//! 2012 crown, wider and lower than the old one, may hang over the road at a height that blocks the view.
//!
//! So after the copies of a stretch are placed, each is settled, in this order (the constants below are the tuned values):
//! 1. **Ground.** The terrain of the section ([`Terrain`]) is read under the copy's foot, nearest to where the placement put it (within
//!    [`MAX_DROP`] below and [`MAX_RISE`] above). A foot more than [`FOOT_TOLERANCE`] off it is put on it, unless the top of a wall or a
//!    building holds it up (a tree line whose crowns begin at a wall's top stands on the wall). The copy keeps its top where it was when it
//!    can (a lowered tree grows, to the scale cap and the squeeze limit; a raised one shrinks by at most 40 percent).
//! 2. **Off the carriageway.** A foot closer than [`TRUNK_MARGIN`] to the edge of a carriageway at its height ([`Roads`]) is moved straight
//!    away from the road to the margin if that is at most [`MAX_SHIFT`] metres and lands on no other road; otherwise the copy is left out.
//!    The moved foot is put on the ground again.
//! 3. **Over the road.** A crown whose underside is less than [`CROWN_CLEARANCE`] above a road it reaches over is narrowed until it reaches
//!    no more than [`OVERHANG`] over the carriageway; a copy that would have to lose more than half its width is left out.
//!
//! A copy that is not in the way of any of these is left exactly as placed, so a stretch beside no road keeps its shape.

use blackbox_modding::content::FitLimits;

use super::clear::Neighbours;
use super::ground::Terrain;
use super::place::{MIN_SCALE, Measure, Placement};
use super::roads::Roads;

/// A foot this close to the ground is on it, metres. (The audit flags a copy beyond it.)
pub const FOOT_TOLERANCE: f32 = 0.3;
/// How far below the placed foot the ground is looked for: the crowns of the 2005 tree lines hang 7 to 8 m up, a canopy's 8 m.
pub const MAX_DROP: f32 = 12.0;
/// How far above it: a cell's lowest vertex under the ground at its middle, on a slope.
pub const MAX_RISE: f32 = 3.0;
/// A raised tree keeps at least this share of its height scale (its top stays where it was, as far as that allows).
const RAISE_KEEP: f32 = 0.6;
/// How far outside the edge of the carriageway the foot of a tree stands, at least, metres.
pub const TRUNK_MARGIN: f32 = 1.0;
/// The furthest a copy is moved to get off a carriageway; beyond it the copy is left out, metres.
pub const MAX_SHIFT: f32 = 5.0;
/// A road whose height is within this of the foot's is the road the tree stands by (a bridge above, or a road below, is another road).
pub const LEVEL_WINDOW: f32 = 3.0;
/// The height the underside of a crown that reaches over a road must clear, metres (a bus is 3.5 m; the audit flags under 3.5).
pub const CROWN_CLEARANCE: f32 = 4.5;
/// A crown may reach this far over the edge of a carriageway at any height (its edge leaves, not a branch over the lane), metres.
pub const OVERHANG: f32 = 0.5;
/// A narrowed crown reaches this much less than the overhang allowed, so that it passes the audit's test, metres.
const NARROW_SLACK: f32 = 0.01;
/// A crown is narrowed to no less than this share of its width; beyond that the copy is left out.
pub const NARROW_FLOOR: f32 = 0.5;
/// A road this far above the underside of a crown is a bridge over the tree, not a road under it, metres.
pub const ROAD_ABOVE_CROWN: f32 = 1.0;

/// What a copy is settled against: the ground, the roads and the objects of the section. Any may be missing (a tile without terrain
/// solids, a world without roads): that step is then skipped.
#[derive(Clone, Copy)]
pub struct Site<'a> {
    pub terrain: Option<&'a Terrain>,
    pub roads: Option<&'a Roads>,
    pub near: &'a Neighbours,
}

/// What settling did, summed over copies.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Settled {
    /// Copies looked at.
    pub copies: usize,
    /// Feet put down onto the ground, and feet put up onto it.
    pub lowered: usize,
    pub raised: usize,
    /// Copies moved off a carriageway, crowns narrowed to clear a road, copies left out.
    pub shifted: usize,
    pub narrowed: usize,
    pub dropped: usize,
}

impl Settled {
    pub fn merge(&mut self, other: Settled) {
        self.copies += other.copies;
        self.lowered += other.lowered;
        self.raised += other.raised;
        self.shifted += other.shifted;
        self.narrowed += other.narrowed;
        self.dropped += other.dropped;
    }
}

/// The ground height under the foot of a copy, nearest to the foot (`None`: no terrain there, or none in the window).
pub fn ground_under(terrain: Option<&Terrain>, foot: [f32; 3]) -> Option<f32> {
    terrain?.surface([foot[0], foot[1]], foot[2], MAX_DROP, MAX_RISE)
}

/// How far the crown of a copy reaches sideways from its foot, metres.
pub fn crown_reach(model: &Measure, at: &Placement) -> f32 {
    0.5 * model.width * at.across
}

/// The height of the underside of the crown of a copy.
pub fn crown_bottom(model: &Measure, at: &Placement) -> f32 {
    at.offset[2] + model.crown_low * at.up
}

/// Settle every copy in `placements` (the copies of one world-space solid; `cap` is the most a copy is scaled up, `place::scale_cap`).
/// A copy that has to go is removed.
pub fn settle(model: &Measure, placements: &mut Vec<Placement>, cap: f32, site: &Site, limits: &FitLimits) -> Settled {
    let mut tally = Settled::default();
    placements.retain_mut(|at| {
        tally.copies += 1;
        let Some(done) = settle_one(model, *at, cap, site, limits, &mut tally) else {
            tally.dropped += 1;
            return false;
        };
        *at = done;
        true
    });
    tally
}

fn settle_one(
    model: &Measure,
    mut at: Placement,
    cap: f32,
    site: &Site,
    limits: &FitLimits,
    tally: &mut Settled,
) -> Option<Placement> {
    stand(model, &mut at, cap, site, limits, tally);
    let Some(roads) = site.roads else { return Some(at) };
    let foot = [at.offset[0], at.offset[1]];
    let level = (at.offset[2] - LEVEL_WINDOW, at.offset[2] + LEVEL_WINDOW);
    if let Some(near) = roads.nearest(foot, TRUNK_MARGIN, level.0, level.1) {
        let push = TRUNK_MARGIN - near.gap;
        if push > MAX_SHIFT {
            return None;
        }
        at.offset[0] += near.away[0] * push;
        at.offset[1] += near.away[1] * push;
        let moved = [at.offset[0], at.offset[1]];
        if roads.nearest(moved, TRUNK_MARGIN - 1e-3, level.0, level.1).is_some() {
            return None;
        }
        tally.shifted += 1;
        stand(model, &mut at, cap, site, limits, tally);
    }
    narrow(model, at, roads, limits, tally)
}

/// Put the foot of `at` on the ground under it (step 1).
fn stand(model: &Measure, at: &mut Placement, cap: f32, site: &Site, limits: &FitLimits, tally: &mut Settled) {
    let foot = at.offset;
    let Some(ground) = ground_under(site.terrain, foot) else { return };
    let off = foot[2] - ground;
    if off.abs() <= FOOT_TOLERANCE {
        return;
    }
    if off > 0.0 && site.near.supports([foot[0], foot[1]], foot[2]) {
        return;
    }
    let top = foot[2] + at.up * model.height;
    let wanted = (top - ground) / model.height.max(0.1);
    at.offset[2] = ground;
    match off > 0.0 {
        true => {
            at.up = wanted.min(cap).min(at.across / limits.squeeze).max(at.up);
            tally.lowered += 1;
        }
        false => {
            at.up = wanted.max(RAISE_KEEP * at.up).max(MIN_SCALE);
            tally.raised += 1;
        }
    }
}

/// Narrow the crown of `at` where it hangs low over a road (step 3); `None` when it would have to lose more than half its width.
fn narrow(
    model: &Measure,
    mut at: Placement,
    roads: &Roads,
    limits: &FitLimits,
    tally: &mut Settled,
) -> Option<Placement> {
    let reach = crown_reach(model, &at);
    let bottom = crown_bottom(model, &at);
    let foot = [at.offset[0], at.offset[1]];
    let Some(near) = roads.nearest(foot, reach, bottom - CROWN_CLEARANCE, bottom + ROAD_ABOVE_CROWN) else {
        return Some(at);
    };
    let over = reach - near.gap;
    if over <= OVERHANG {
        return Some(at);
    }
    let allowed = (near.gap + OVERHANG - NARROW_SLACK).max(0.0);
    let share = allowed / reach;
    if share < NARROW_FLOOR {
        return None;
    }
    at.across *= share;
    at.up = at.up.min(at.across / limits.squeeze);
    at.base = at.base.min(at.across);
    tally.narrowed += 1;
    Some(at)
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
