//! Bounded route scoring for a target vehicle. Spec: `docs/specs/ai-road-target-tracker.md`.

use glam::Vec3;

use super::spot::{Spot, snap};
use crate::{Body, LaneType, NavKind, PathType, RoadNav, RoadNetwork, SegmentFilter, SegmentIndex, centre_line, flags};

const MAX_WALKS: usize = 32;
const MAX_ENDS: usize = 10;
// Safety bound for corrupt cyclic chains, independent of the native branch budget.
const MAX_CHAIN: usize = 4096;

pub(super) struct Found {
    pub current: RoadNav,
    pub future: RoadNav,
    pub far: RoadNav,
}

struct Start {
    score: f32,
    nav: RoadNav,
}

struct Search<'a> {
    net: &'a RoadNetwork,
    direction: Vec3,
    future_point: Vec3,
    target_point: Vec3,
    ratio: f32,
    previous: [Option<u16>; 2],
    walks: usize,
    ends: usize,
    best_score: f32,
    best: Option<Found>,
}

pub(super) fn walk(
    net: &RoadNetwork,
    index: &SegmentIndex,
    body: Body,
    previous: Option<(u16, usize)>,
) -> Option<Found> {
    let mut velocity = body.velocity;
    let mut speed = velocity.length();
    if speed < 1.0 {
        velocity = Vec3::new(body.forward.x, 0.0, body.forward.y);
        speed = velocity.length();
    }
    let direction = velocity.try_normalize()?;
    let future_distance = speed.max(2.0);
    let target_distance = (speed * 2.0).max(90.0);
    let mut starts = Vec::new();
    for segment in index.near(body.position, 25.0) {
        let seg = net.segment(segment);
        let t = centre_line(net, segment, 1).closest_t(body.position, seg.length);
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter::default());
        nav.path_type = PathType::Cop;
        nav.init_at_segment(net, segment, t, body.position, direction, false);
        snap(net, &mut nav, body.position);
        let delta = body.position - nav.position;
        let forward = Vec3::new(nav.forward.x, 0.0, nav.forward.z).normalize_or_zero();
        let score = (delta.x.hypot(delta.z) - 2.0).max(0.0)
            + 3.0 * (delta.y.abs() - 2.0).max(0.0)
            + 2.0 * (1.0 - direction.dot(forward));
        starts.push(Start { score, nav });
    }
    starts.sort_by(|a, b| a.score.total_cmp(&b.score).then(a.nav.segment.cmp(&b.nav.segment)));
    let previous_segment = previous.map(|p| p.0);
    let successor = previous.and_then(|(segment, node)| {
        let segment = net.segments.get(usize::from(segment))?;
        net.plain_segment_at(*segment.nodes.get(node)?, previous_segment?)
    });
    let mut search = Search {
        net,
        direction,
        future_point: body.position + direction * future_distance,
        target_point: body.position + direction * target_distance,
        ratio: future_distance / target_distance,
        previous: [previous_segment, successor],
        walks: 0,
        ends: 0,
        best_score: f32::MAX,
        best: None,
    };
    for start in starts {
        if search.walks >= MAX_WALKS || search.ends >= MAX_ENDS || start.score >= search.best_score {
            break;
        }
        search.paths(&start, Spot::of(&start.nav), future_distance, target_distance, false, None);
    }
    search.best
}

impl Search<'_> {
    fn paths(
        &mut self,
        start: &Start,
        mut spot: Spot,
        mut future_distance: f32,
        mut target_distance: f32,
        mut cop_penalty: bool,
        mut future: Option<Spot>,
    ) {
        if self.walks >= MAX_WALKS || self.ends >= MAX_ENDS {
            return;
        }
        self.walks += 1;
        let net = self.net;
        for _ in 0..MAX_CHAIN {
            let segment = net.segment(spot.segment);
            if segment.is_one_way() && spot.node == 0 {
                return;
            }
            if !segment.length.is_finite() || segment.length <= 0.0 {
                return;
            }
            let allowed = SegmentFilter { cop: true, ..Default::default() }.allows(segment);
            cop_penalty |= !allowed;
            let future_t = spot.t + future_distance / segment.length;
            if future_distance > 0.0 && future_t <= 1.0 {
                future = Some(Spot { t: future_t, lane: 0, ..spot });
            }
            let target_t = spot.t + target_distance / segment.length;
            if target_t <= 1.0 {
                if let Some(future) = future {
                    self.evaluate(start, future, Spot { t: target_t, lane: 0, ..spot }, cop_penalty);
                }
                return;
            }
            let spent = (1.0 - spot.t) * segment.length;
            future_distance -= spent;
            target_distance -= spent;
            let node = segment.nodes[spot.node];
            if let Some(next) = net.plain_segment_at(node, spot.segment) {
                let new_segment = net.segment(next);
                spot = Spot { segment: next, node: usize::from(new_segment.nodes[0] == node), t: 0.0, lane: 0 };
                continue;
            }
            for &next in &net.node(node).segments {
                if self.walks >= MAX_WALKS || self.ends >= MAX_ENDS {
                    break;
                }
                let new_segment = net.segment(next);
                if next == spot.segment
                    || new_segment.has(flags::CROSSES_BARRIER | flags::CROSSES_DRIVE_THROUGH_BARRIER)
                {
                    continue;
                }
                let next_spot =
                    Spot { segment: next, node: usize::from(new_segment.nodes[0] == node), t: 0.0, lane: 0 };
                self.paths(start, next_spot, future_distance, target_distance, cop_penalty, future);
            }
            return;
        }
    }

    fn evaluate(&mut self, start: &Start, future: Spot, target: Spot, cop_penalty: bool) {
        self.ends += 1;
        let penalty = match cop_penalty {
            true => 5.0,
            false => 0.0,
        };
        let mut score = start.score + penalty;
        if score >= self.best_score {
            return;
        }
        let target = target.nav(self.net, Some(self.target_point));
        let error = |displacement: Vec3| {
            let planar = Vec3::new(displacement.x, 0.0, displacement.z).normalize_or_zero();
            1.0 - planar.dot(self.direction)
        };
        score += error(target.position - start.nav.position) * 16.0;
        if score >= self.best_score {
            return;
        }
        let local = self.future_point.lerp(start.nav.position.lerp(target.position, self.ratio), self.ratio);
        let future = future.nav(self.net, Some(local));
        score += error(future.position - start.nav.position) * 24.0 + error(target.forward) * 2.0;
        if !self.previous.contains(&Some(future.segment)) {
            score += 2.0;
        }
        if score >= self.best_score {
            return;
        }
        self.best_score = score;
        self.best = Some(Found { current: start.nav.clone(), future, far: target });
    }
}
