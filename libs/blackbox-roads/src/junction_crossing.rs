//! Where the traffic of a signalled junction crosses the path of a car turning into it, and how soon an oncoming car
//! reaches that point. Rewrite extension for the oncoming-gap rule (`docs/specs/ai-traffic-obstacles.md` §10.2); the
//! original has no yield rule. The road data gives no lane-level crossings, so the crossing point is the centre of the
//! junction: the mean of its nodes. That is exact for a four-way junction and an estimate for a T junction.

use glam::{Vec2, Vec3};

use crate::{Body, RoadNetwork, SignalController, time_to_closest};

/// A car whose heading (unit, planar) has a dot product with ours at or above this value (cos 45 degrees) goes the same
/// way as we do: it is behind us, not oncoming, and it does not count.
pub const SAME_WAY: f32 = 0.707;

/// The point where the cars of signalled approach `approach` (an index into [`SignalController::approaches`]) cross the
/// other roads of its junction: the mean of the junction's node positions. `None` when there is no such approach.
pub fn crossing_point(net: &RoadNetwork, controller: &SignalController, approach: usize) -> Option<Vec3> {
    let junction = controller.junctions().get(controller.approaches().get(approach)?.junction)?;
    let count = junction.nodes.len();
    if count == 0 {
        return None;
    }
    let sum = junction.nodes.iter().fold(Vec3::ZERO, |sum, &node| sum + net.node(node).position);
    Some(sum / count as f32)
}

/// The seconds until the first car of `bodies` that is not going the same way as `heading` (a unit planar vector)
/// reaches `point` at its closest approach. `None` when no such car is moving towards the point.
pub fn earliest_oncoming(point: Vec3, heading: Vec2, bodies: impl IntoIterator<Item = Body>) -> Option<f32> {
    let point = Vec2::new(point.x, point.z);
    bodies
        .into_iter()
        .filter(|body| body.forward.dot(heading) < SAME_WAY)
        .filter_map(|body| {
            let position = Vec2::new(body.position.x, body.position.z);
            let velocity = Vec2::new(body.velocity.x, body.velocity.z);
            time_to_closest(point, position, velocity)
        })
        .reduce(f32::min)
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::{Road, RoadNode, RoadProfile, RoadSegment, Zone, zone};

    fn car(position: Vec3, velocity: Vec3, forward: Vec2) -> Body {
        Body { position, velocity, forward, half_width: 0.9, half_length: 2.2 }
    }

    /// A T junction. Nodes 0, 1 and 2 are at the junction (10 m from its middle, west, east and south); segment `i` is
    /// the plain arm of node `i`, driven towards it from node `i + 3`; segments 3 to 5 are the decision segments that
    /// join the three nodes.
    fn t_junction() -> RoadNetwork {
        let arms = [Vec3::NEG_X, Vec3::X, Vec3::NEG_Z];
        let segment = |nodes: [u16; 2], flags: u16| RoadSegment {
            nodes,
            length: 100.0,
            road: Some(0),
            flags,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        };
        let mut nodes: Vec<RoadNode> = arms
            .iter()
            .enumerate()
            .map(|(i, d)| RoadNode { position: *d * 10.0, profile: 0, segments: vec![i as u16] })
            .collect();
        nodes.extend(arms.iter().enumerate().map(|(i, d)| RoadNode {
            position: *d * 110.0,
            profile: 0,
            segments: vec![i as u16],
        }));
        let mut segments: Vec<RoadSegment> = (0..3).map(|i| segment([i + 3, i], 0)).collect();
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            segments.push(segment([a, b], crate::flags::DECISION | crate::flags::INTERSECTION));
            let index = segments.len() as u16 - 1;
            nodes[usize::from(a)].segments.push(index);
            nodes[usize::from(b)].segments.push(index);
        }
        let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
        RoadNetwork {
            nodes,
            segments,
            profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
            roads: vec![Road { scale: 1.0, speech_id: 0 }],
        }
    }

    #[test]
    fn the_crossing_point_is_the_mean_of_the_junction_nodes() {
        let net = t_junction();
        let controller = SignalController::new(&net);
        assert!(!controller.approaches().is_empty(), "the test junction is signalled");
        let point = crossing_point(&net, &controller, 0).expect("approach 0 exists");
        // The nodes are at (-10, 0, 0), (10, 0, 0) and (0, 0, -10): their mean is (0, 0, -10 / 3).
        assert!(point.x.abs() < 1e-4 && point.y.abs() < 1e-4, "{point:?}");
        assert!((point.z + 10.0 / 3.0).abs() < 1e-4, "{point:?}");
    }

    #[test]
    fn an_approach_that_does_not_exist_has_no_crossing_point() {
        let net = t_junction();
        let controller = SignalController::new(&net);
        assert_eq!(crossing_point(&net, &controller, 99), None);
    }

    #[test]
    fn an_oncoming_car_counts_with_the_time_to_reach_the_point() {
        // Heading east (+X). An opposing car 25 m east of the point, driving west at 10 m/s, is there in 2.5 s.
        let oncoming = car(Vec3::new(25.0, 0.0, 0.0), Vec3::new(-10.0, 0.0, 0.0), Vec2::NEG_X);
        let t = earliest_oncoming(Vec3::ZERO, Vec2::X, [oncoming]).expect("oncoming");
        assert!((t - 2.5).abs() < 1e-4, "{t}");
    }

    #[test]
    fn a_car_going_the_same_way_is_behind_us_and_does_not_count() {
        // Ahead of us in our own direction, driving away is not a threat; behind us and closing is still ignored.
        let behind = car(Vec3::new(-30.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0), Vec2::X);
        assert_eq!(earliest_oncoming(Vec3::ZERO, Vec2::X, [behind]), None);
    }

    #[test]
    fn a_car_crossing_at_a_right_angle_counts() {
        // A car from the south (-Z side) driving north at 10 m/s, 25 m from the point: it crosses in 2.5 s.
        let crossing = car(Vec3::new(0.0, 0.0, -25.0), Vec3::new(0.0, 0.0, 10.0), Vec2::Y);
        let t = earliest_oncoming(Vec3::ZERO, Vec2::X, [crossing]).expect("crossing car");
        assert!((t - 2.5).abs() < 1e-4, "{t}");
    }

    #[test]
    fn a_standing_car_and_a_car_moving_away_do_not_count() {
        let standing = car(Vec3::new(20.0, 0.0, 0.0), Vec3::ZERO, Vec2::NEG_X);
        let leaving = car(Vec3::new(20.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0), Vec2::X);
        assert_eq!(earliest_oncoming(Vec3::ZERO, Vec2::X, [standing, leaving]), None);
    }

    #[test]
    fn the_earliest_of_several_oncoming_cars_is_the_one_that_counts() {
        let far = car(Vec3::new(40.0, 0.0, 0.0), Vec3::new(-10.0, 0.0, 0.0), Vec2::NEG_X);
        let near = car(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, -10.0), Vec2::NEG_Y);
        let t = earliest_oncoming(Vec3::ZERO, Vec2::X, [far, near]).expect("two cars");
        assert!((t - 1.0).abs() < 1e-4, "{t}");
    }
}
