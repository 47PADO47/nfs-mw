//! The GPS route as the HUD reads it (`hud/state/nav.rs`): the route's road segments as one polyline in render space,
//! the points where one segment meets the next, and the next turn from the car.

use blackbox_roads::{RoadNetwork, centre_line};

use super::super::{WorldScene, space};
use crate::hud::{NavRoute, next_turn};

/// Points taken along each road segment of the route (its two ends included).
const SAMPLES: usize = 8;

impl WorldScene {
    /// The lead's GPS route with its next turn; `None` while no route is set.
    pub(in crate::scenes::world) fn nav_route(&self) -> Option<NavRoute> {
        let route = self.guide.route.as_ref()?;
        let (net, _) = self.traffic.as_ref()?.road_graph();
        let car = self.players.lead()?.pose().position;
        let pieces = route.path().iter().map(|&segment| segment_points(net, segment)).collect();
        let (path, junctions) = chain(pieces);
        let target = space::to_render(route.target().to_array());
        Some(NavRoute { next_turn: next_turn(&path, &junctions, [car.x, car.y]), path, target: [target.x, target.y] })
    }
}

/// The centre line of road segment `segment`, render space (x, y), in the segment's stored direction.
fn segment_points(net: &RoadNetwork, segment: u16) -> Vec<[f32; 2]> {
    let line = centre_line(net, segment, 1);
    (0..=SAMPLES)
        .map(|i| {
            let at = space::to_render(line.position(i as f32 / SAMPLES as f32).to_array());
            [at.x, at.y]
        })
        .collect()
}

/// The route's segments, start first, as one polyline: each segment turned to run on from the end of the one before
/// (the first towards the second), and the indices of the points where one meets the next. A segment of fewer than two
/// points is left out.
fn chain(pieces: Vec<Vec<[f32; 2]>>) -> (Vec<[f32; 2]>, Vec<usize>) {
    let pieces: Vec<Vec<[f32; 2]>> = pieces.into_iter().filter(|piece| piece.len() >= 2).collect();
    let mut path: Vec<[f32; 2]> = Vec::new();
    let mut junctions = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        let (first, last) = (piece[0], piece[piece.len() - 1]);
        let reversed = match path.last() {
            Some(end) => gap(*end, first) > gap(*end, last),
            None => pieces.get(i + 1).is_some_and(|next| nearest_end(first, next) < nearest_end(last, next)),
        };
        let mut points = piece.clone();
        if reversed {
            points.reverse();
        }
        if path.is_empty() {
            path.extend(points);
            continue;
        }
        junctions.push(path.len() - 1);
        path.extend(points.into_iter().skip(1));
    }
    (path, junctions)
}

/// The distance from `point` to the nearer end of `piece`.
fn nearest_end(point: [f32; 2], piece: &[[f32; 2]]) -> f32 {
    let ends = [piece[0], piece[piece.len() - 1]];
    ends.iter().map(|end| gap(point, *end)).fold(f32::INFINITY, f32::min)
}

fn gap(a: [f32; 2], b: [f32; 2]) -> f32 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_in_any_direction_chain_into_one_line() {
        // Three segments of an L-shaped route, the first and the last stored against the way the route runs.
        let pieces =
            vec![vec![[50.0, 0.0], [0.0, 0.0]], vec![[50.0, 0.0], [100.0, 0.0]], vec![[100.0, 80.0], [100.0, 0.0]]];
        let (path, junctions) = chain(pieces);
        assert_eq!(path, vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0], [100.0, 80.0]]);
        assert_eq!(junctions, vec![1, 2], "the points where the segments meet");
    }

    #[test]
    fn the_chained_route_turns_at_its_junction() {
        let pieces = vec![vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]], vec![[100.0, 0.0], [100.0, 50.0]]];
        let (path, junctions) = chain(pieces);
        let turn = next_turn(&path, &junctions, [0.0, 0.0]).unwrap();
        assert_eq!(turn.direction, crate::hud::TurnDirection::Left);
        assert!((turn.distance_m - 100.0).abs() < 1e-3);
    }

    #[test]
    fn empty_and_single_point_segments_are_left_out() {
        let (path, junctions) = chain(vec![vec![], vec![[1.0, 1.0]], vec![[0.0, 0.0], [5.0, 0.0]]]);
        assert_eq!(path, vec![[0.0, 0.0], [5.0, 0.0]]);
        assert!(junctions.is_empty());
        assert_eq!(chain(Vec::new()), (Vec::new(), Vec::new()));
    }
}
