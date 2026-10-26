//! The coarse convex shape a body rolls on: its side silhouette, mirrored across the centre plane.
//!
//! The mesh-versus-ground point cloud of a stock car is 8 points on each side, tracing the side view of the body
//! (`docs/formats/collision.md`, "Bounds"). The cloud here is made the same way: the convex hull of the side view
//! (z along the body, y up), thinned to a handful of corners, each corner given the half width the body has there.

use crate::math::Vec3;

/// The corners of the convex hull of `points`, counter-clockwise, without collinear points. Fewer than 3 distinct
/// points give those points.
pub(super) fn convex_hull(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut sorted: Vec<[f32; 2]> = points.iter().copied().filter(|p| p[0].is_finite() && p[1].is_finite()).collect();
    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let cross = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut hull: Vec<[f32; 2]> = Vec::with_capacity(sorted.len() * 2);
    for pass in 0..2 {
        let start = hull.len();
        let walk: Box<dyn Iterator<Item = &[f32; 2]>> = match pass {
            0 => Box::new(sorted.iter()),
            _ => Box::new(sorted.iter().rev()),
        };
        for &p in walk {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// `polygon` (convex, counter-clockwise) thinned to at most `max` corners: the corner whose removal gives up the
/// least area goes first. The four extreme corners (lowest and highest on each axis) are kept so the silhouette
/// stays as long and as tall as the body.
pub(super) fn thin(mut polygon: Vec<[f32; 2]>, max: usize) -> Vec<[f32; 2]> {
    let extreme = |p: &[f32; 2], all: &[[f32; 2]]| {
        let fold = |axis: usize, pick: fn(f32, f32) -> f32, init: f32| all.iter().map(|q| q[axis]).fold(init, pick);
        p[0] == fold(0, f32::min, f32::MAX)
            || p[0] == fold(0, f32::max, f32::MIN)
            || p[1] == fold(1, f32::min, f32::MAX)
            || p[1] == fold(1, f32::max, f32::MIN)
    };
    let keep: Vec<[f32; 2]> = polygon.iter().filter(|p| extreme(p, &polygon)).copied().collect();
    let max = max.max(keep.len()).max(3);
    while polygon.len() > max {
        let n = polygon.len();
        let area = |i: usize| {
            let (a, b, c) = (polygon[(i + n - 1) % n], polygon[i], polygon[(i + 1) % n]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
        };
        let cheapest = (0..n).filter(|&i| !keep.contains(&polygon[i])).min_by(|&a, &b| area(a).total_cmp(&area(b)));
        let Some(cheapest) = cheapest else { break };
        polygon.remove(cheapest);
    }
    polygon
}

/// The mirrored silhouette cloud of `points` (physics axes: x across, y up, z along), as `[x, y, z]` with x measured
/// from `centre_x`. `corners` is how many silhouette corners each side gets at most; the result has twice as many
/// points. Each corner's half width is the largest `|x - centre_x|` among the points within `reach` metres of it in
/// the side view (the nearest point's, when none is that close), so the cloud follows the body's width along its
/// length.
pub(super) fn silhouette(points: &[Vec3], centre_x: f32, corners: usize, reach: f32) -> Vec<Vec3> {
    let side: Vec<[f32; 2]> = points.iter().map(|p| [p[2], p[1]]).collect();
    let outline = thin(convex_hull(&side), corners);
    let mut right = Vec::with_capacity(outline.len());
    let mut left = Vec::with_capacity(outline.len());
    for corner in outline {
        let distance = |p: &Vec3| ((p[2] - corner[0]).powi(2) + (p[1] - corner[1]).powi(2)).sqrt();
        let near = points.iter().filter(|p| distance(p) <= reach).map(|p| (p[0] - centre_x).abs()).fold(0.0, f32::max);
        let half = match near > 0.0 {
            true => near,
            false => points
                .iter()
                .min_by(|a, b| distance(a).total_cmp(&distance(b)))
                .map_or(0.0, |p| (p[0] - centre_x).abs()),
        };
        right.push([centre_x + half, corner[1], corner[0]]);
        left.push([centre_x - half, corner[1], corner[0]]);
    }
    right.extend(left);
    right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hull_of_a_square_with_inner_points_is_its_four_corners() {
        let mut points = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        points.extend([[1.0, 1.0], [0.5, 1.5], [1.0, 0.0]]);
        let hull = convex_hull(&points);
        assert_eq!(hull.len(), 4);
        for corner in [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]] {
            assert!(hull.contains(&corner), "{corner:?}");
        }
    }

    #[test]
    fn the_hull_is_counter_clockwise() {
        let hull = convex_hull(&[[0.0, 0.0], [4.0, 0.0], [4.0, 1.0], [0.0, 1.0]]);
        let area: f32 = (0..hull.len())
            .map(|i| {
                let (a, b) = (hull[i], hull[(i + 1) % hull.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum();
        assert!(area > 0.0);
    }

    #[test]
    fn thinning_keeps_the_extremes_and_the_count() {
        let circle: Vec<[f32; 2]> = (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * std::f32::consts::TAU;
                [a.cos() * 2.0, a.sin()]
            })
            .collect();
        let thinned = thin(convex_hull(&circle), 8);
        assert_eq!(thinned.len(), 8);
        let max_x = thinned.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((max_x - 2.0).abs() < 1e-4);
    }

    #[test]
    fn the_silhouette_is_mirrored_and_follows_the_width() {
        // A wide tail (|x| = 1) and a narrow nose (|x| = 0.4) of a 4 m long, 1 m tall body.
        let points = vec![
            [1.0, 0.0, -2.0],
            [-1.0, 0.0, -2.0],
            [1.0, 1.0, -2.0],
            [-1.0, 1.0, -2.0],
            [0.4, 0.0, 2.0],
            [-0.4, 0.0, 2.0],
            [0.4, 0.5, 2.0],
            [-0.4, 0.5, 2.0],
        ];
        let cloud = silhouette(&points, 0.0, 6, 0.3);
        assert!(cloud.len() % 2 == 0 && cloud.len() >= 6);
        let (right, left) = cloud.split_at(cloud.len() / 2);
        for (r, l) in right.iter().zip(left) {
            assert_eq!((r[1], r[2]), (l[1], l[2]));
            assert_eq!(r[0], -l[0]);
        }
        let tail = right.iter().find(|p| p[2] < -1.9).unwrap();
        let nose = right.iter().find(|p| p[2] > 1.9).unwrap();
        assert_eq!((tail[0], nose[0]), (1.0, 0.4));
    }
}
