//! Track path barriers (chunk `0x3414D` of the track's metadata file): lines on the map that AI routes and the GPS may not
//! cross while the scenery group they belong to is switched on. Format: `docs/formats/road-network.md` ("Track path
//! barriers"); rules: `docs/specs/ai-road-network.md` section 5.3 and `docs/specs/race-barriers.md` section 5.
//!
//! The lines are in the track's 2D frame ([`crate::to_zone_space`]).

use blackbox_chunk::ids;
use glam::{Vec2, Vec3};

use crate::bytes::Reader;
use crate::error::malformed;
use crate::{Result, RoadNetwork, SegmentIndex, centre_line, flags, to_zone_space};

/// Bytes per record.
pub const BARRIER_LEN: usize = 24;

/// One barrier line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackBarrier {
    pub a: Vec2,
    pub b: Vec2,
    /// Runtime in the original (0 in the file): the barrier's group is on.
    pub enabled: bool,
    /// Runtime in the original (0 in the file): the group's drive-through byte, copied when it is switched on.
    pub drive_through: bool,
    /// Meaning not followed.
    pub left_handed: bool,
    /// `bStringHash` of the scenery group name the barrier belongs to.
    pub group_hash: u32,
}

/// Every record of a barrier chunk payload.
pub fn parse_track_barriers(payload: &[u8]) -> Result<Vec<TrackBarrier>> {
    if !payload.len().is_multiple_of(BARRIER_LEN) {
        return Err(malformed("track barriers", format!("{} bytes is not a multiple of {BARRIER_LEN}", payload.len())));
    }
    let r = Reader::new(payload, "track barriers");
    (0..payload.len() / BARRIER_LEN)
        .map(|i| {
            let at = i * BARRIER_LEN;
            Ok(TrackBarrier {
                a: Vec2::new(r.f32(at)?, r.f32(at + 4)?),
                b: Vec2::new(r.f32(at + 8)?, r.f32(at + 12)?),
                enabled: r.u8(at + 16)? != 0,
                drive_through: r.u8(at + 18)? != 0,
                left_handed: r.u8(at + 19)? != 0,
                group_hash: r.u32(at + 20)?,
            })
        })
        .collect()
}

/// The barriers in `data` (the track's world metadata file); none when it has no barrier chunk. The chunk is a
/// top-level chunk of the file, beside the track path manager rather than in it.
pub fn read_track_barriers(data: &[u8]) -> Result<Vec<TrackBarrier>> {
    let Some(chunk) = blackbox_chunk::find(data, ids::TRACK_PATH_BARRIERS) else { return Ok(Vec::new()) };
    parse_track_barriers(chunk.payload)
}

/// One switched-on barrier line, as the road flags see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveBarrier {
    pub a: Vec2,
    pub b: Vec2,
    /// Sets `CROSSES_DRIVE_THROUGH_BARRIER` instead of `CROSSES_BARRIER`.
    pub drive_through: bool,
}

/// Whether segment `segment`'s centre line crosses the line `a`-`b`: the curve is cut into about one piece per
/// 10 m, at least 4, and each piece is tested against the line in the 2D frame.
pub fn segment_crosses(net: &RoadNetwork, segment: u16, a: Vec2, b: Vec2) -> bool {
    let curve = centre_line(net, segment, 1);
    let pieces = ((net.segment(segment).length / 10.0).ceil() as usize).max(4);
    let point = |i: usize| to_zone_space(curve.position(i as f32 / pieces as f32));
    let mut previous = point(0);
    for i in 1..=pieces {
        let next = point(i);
        if lines_cross(a, b, previous, next) {
            return true;
        }
        previous = next;
    }
    false
}

/// Whether the 2D segments `a`-`b` and `c`-`d` share a point.
fn lines_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let side = |p: Vec2, q: Vec2, r: Vec2| (q - p).perp_dot(r - p);
    let (d1, d2) = (side(a, b, c), side(a, b, d));
    let (d3, d4) = (side(c, d, a), side(c, d, b));
    d1 * d2 <= 0.0 && d3 * d4 <= 0.0 && (d1 != 0.0 || d2 != 0.0 || collinear_overlap(a, b, c, d))
}

/// Two collinear segments overlap when their projections on the line do.
fn collinear_overlap(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let axis = (b - a).try_normalize().unwrap_or(Vec2::X);
    let (p0, p1) = (a.dot(axis).min(b.dot(axis)), a.dot(axis).max(b.dot(axis)));
    let (q0, q1) = (c.dot(axis).min(d.dot(axis)), c.dot(axis).max(d.dot(axis)));
    p0 <= q1 && q0 <= p1
}

/// Recomputes the barrier flags of every segment: both flags are cleared, then every segment whose centre line
/// crosses an active barrier gets `CROSSES_BARRIER` (or `CROSSES_DRIVE_THROUGH_BARRIER` for a drive-through one),
/// except segments of the `exempt` roads (the roads at the race start, or the free-roam exemption point).
pub fn mark_barrier_crossings(
    net: &mut RoadNetwork,
    index: &SegmentIndex,
    barriers: &[ActiveBarrier],
    exempt: &[u16],
) -> usize {
    let both = flags::CROSSES_BARRIER | flags::CROSSES_DRIVE_THROUGH_BARRIER;
    for segment in &mut net.segments {
        segment.flags &= !both;
    }
    let mut marked = 0;
    for barrier in barriers {
        let middle = (barrier.a + barrier.b) * 0.5;
        let reach = (barrier.b - barrier.a).length() * 0.5 + 1.0;
        let flag = match barrier.drive_through {
            true => flags::CROSSES_DRIVE_THROUGH_BARRIER,
            false => flags::CROSSES_BARRIER,
        };
        let centre = Vec3::new(-middle.y, 0.0, middle.x);
        for s in index.near(centre, reach) {
            let road = net.segment(s).road;
            if road.is_some_and(|r| exempt.contains(&r)) || !segment_crosses(net, s, barrier.a, barrier.b) {
                continue;
            }
            marked += usize::from(!net.segment(s).has(both));
            net.segments[usize::from(s)].flags |= flag;
        }
    }
    marked
}

#[cfg(test)]
#[path = "barriers_tests.rs"]
mod tests;
