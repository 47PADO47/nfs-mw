//! Building a whole route, with a fallback for each leg whose search fails. The barrier lines of the route close the
//! roads they cross while the legs are searched, and the network's barrier flags are clear again afterwards.

use glam::Vec3;

use super::{Goal, LegPath, Route, RouteBuilder, build_leg};
use crate::{ActiveBarrier, PathType, RoadNetwork, SegmentIndex, flags, mark_barrier_crossings};

/// Metres around a leg's two ends inside which the lines are lifted for its second try: a goal a line passes close to.
pub const END_REACH: f32 = 60.0;
/// Spacing of the points of a leg's centre line that [`build_route`] hands to its check, metres.
pub const CHECK_SPACING: f32 = 2.0;

/// How a leg's road was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegSource {
    /// A route search with the barrier lines closing the roads they cross.
    Closed,
    /// The same with the lines lifted near the leg's ends.
    OpenEnds,
    /// A plain racer search with no lines: it may turn round where a route search may not.
    Open,
    /// No road: the leg is a straight line.
    Missing,
}

/// The route from `start` through `goals`. Each leg tries, in turn: a [`PathType::RaceRoute`] search with `lines`
/// marked (sparing the roads in `exempt`), the same with the marks lifted within [`END_REACH`] of its ends, and a
/// [`PathType::Racer`] search with no marks; else it is missing. A road found counts only when `accepts(leg, points)`
/// holds for its centre line sampled every [`CHECK_SPACING`] metres (for example: it passes through its goal). A leg
/// between the same two goals as an earlier one (the next lap) reuses that leg's road. Every barrier flag of `net` is
/// clear when this returns.
pub fn build_route(
    net: &mut RoadNetwork,
    index: &SegmentIndex,
    start: Goal,
    goals: &[Goal],
    (lines, exempt): (&[ActiveBarrier], &[u16]),
    mut accepts: impl FnMut(usize, &[Vec3]) -> bool,
) -> (Route, Vec<LegSource>) {
    let both = flags::CROSSES_BARRIER | flags::CROSSES_DRIVE_THROUGH_BARRIER;
    mark_barrier_crossings(net, index, lines, exempt);
    let marked: Vec<u16> = net.segments.iter().map(|s| s.flags & both).collect();
    let mut legs: Vec<(Goal, Goal, Option<LegPath>, LegSource)> = Vec::new();
    for (leg, &to) in goals.iter().enumerate() {
        let from = leg.checked_sub(1).map_or(start, |before| goals[before]);
        if let Some((_, _, path, source)) = legs.iter().find(|(f, t, ..)| *f == from && *t == to).cloned() {
            legs.push((from, to, path, source));
            continue;
        }
        let mut accepted = |path: &LegPath, net: &RoadNetwork| accepts(leg, &centre_points(net, from, to, path));
        let tries = [
            (LegSource::Closed, PathType::RaceRoute),
            (LegSource::OpenEnds, PathType::RaceRoute),
            (LegSource::Open, PathType::Racer),
        ];
        let mut found = (None, LegSource::Missing);
        for (source, path_type) in tries {
            restore(net, &marked, both);
            match source {
                LegSource::OpenEnds => lift(net, index, [from.position, to.position], both),
                LegSource::Open => lift_all(net, both),
                _ => {}
            }
            let Some(path) = build_leg(net, index, from, to, path_type) else { continue };
            if accepted(&path, net) {
                found = (Some(path), source);
                break;
            }
        }
        legs.push((from, to, found.0, found.1));
    }
    lift_all(net, both);
    let mut builder = RouteBuilder::new(net);
    for (from, to, path, _) in &legs {
        builder.push(*from, *to, path.as_ref());
    }
    (builder.finish(), legs.into_iter().map(|(.., source)| source).collect())
}

/// The centre line of one leg's road, a point every [`CHECK_SPACING`] metres and its end.
fn centre_points(net: &RoadNetwork, from: Goal, to: Goal, path: &LegPath) -> Vec<Vec3> {
    let mut builder = RouteBuilder::new(net);
    builder.push(from, to, Some(path));
    let route = builder.finish();
    let count = (route.length / CHECK_SPACING).ceil() as usize;
    (0..=count).filter_map(|i| route.point_at(net, i as f32 * CHECK_SPACING).map(|(p, _)| p)).collect()
}

fn restore(net: &mut RoadNetwork, marked: &[u16], both: u16) {
    for (segment, &bits) in net.segments.iter_mut().zip(marked) {
        segment.flags = (segment.flags & !both) | bits;
    }
}

fn lift(net: &mut RoadNetwork, index: &SegmentIndex, ends: [Vec3; 2], both: u16) {
    for end in ends {
        for s in index.near(end, END_REACH) {
            net.segments[usize::from(s)].flags &= !both;
        }
    }
}

fn lift_all(net: &mut RoadNetwork, both: u16) {
    for segment in &mut net.segments {
        segment.flags &= !both;
    }
}
