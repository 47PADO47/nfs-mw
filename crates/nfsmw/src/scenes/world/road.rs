//! Finding a road to put a car on: the collision surface type tells asphalt from pavement and grass.

use blackbox_collision::CollisionWorld;
use glam::Vec3;

use super::space;

/// `simsurface` keys of the surfaces cars drive on (`docs/formats/collision.md`): asphalt, concrete.
const ROAD_SURFACES: [u32; 2] = [0x19DB_2F1E, 0x6CA2_6F9B];
/// Steeper than this (the normal's vertical part) is not a road.
const FLAT: f32 = 0.97;
/// Ring spacing and point spacing of the search, metres.
const SEARCH_STEP: f32 = 6.0;
const SEARCH_RADIUS: f32 = 400.0;
/// Directions tried for the road's heading, and how far along each is followed.
const HEADINGS: usize = 24;
const RUN_STEP: f32 = 2.0;
const RUN_LENGTH: f32 = 80.0;
/// A step up or down bigger than this ends a run (a kerb, a wall).
const MAX_RISE: f32 = 0.5;
/// How far sideways the road is measured to centre on it.
const HALF_WIDTH_PROBE: f32 = 14.0;

/// What is under a point of the map (render space x, y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
    pub height: f32,
    pub road: bool,
}

/// A place to put a car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spawn {
    pub position: Vec3,
    /// Heading in radians from +X.
    pub heading: f32,
}

/// The road surface below `top_z` at (x, y), as the collision world sees it.
pub fn probe(collision: &CollisionWorld, x: f32, y: f32, top_z: f32) -> Option<Surface> {
    let [px, _, pz] = space::to_physics(Vec3::new(x, y, 0.0));
    let hit = collision.ground(px, pz, top_z, top_z - 160.0)?;
    Some(Surface { height: hit.point[1], road: ROAD_SURFACES.contains(&hit.surface_hash) && hit.normal[1] > FLAT })
}

/// The nearest road to `near` (map x, y), searched outwards in rings, with the heading it runs in and
/// a position centred between its edges. `probe` looks at one map point.
pub fn find(near: [f32; 2], probe: impl Fn(f32, f32) -> Option<Surface>) -> Option<Spawn> {
    let ring_points = |radius: f32| {
        let n = if radius == 0.0 { 1 } else { ((std::f32::consts::TAU * radius) / SEARCH_STEP).ceil() as usize };
        (0..n).map(move |i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            [near[0] + radius * a.cos(), near[1] + radius * a.sin()]
        })
    };
    let mut radius = 0.0;
    while radius <= SEARCH_RADIUS {
        for [x, y] in ring_points(radius) {
            if let Some(Surface { road: true, .. }) = probe(x, y) {
                return Some(settle([x, y], &probe));
            }
        }
        radius += SEARCH_STEP;
    }
    None
}

/// Heading along the longest run of road from `at`, and a spot centred across it.
fn settle(at: [f32; 2], probe: &impl Fn(f32, f32) -> Option<Surface>) -> Spawn {
    let run = |from: [f32; 2], heading: f32, limit: f32| {
        let (dx, dy) = (heading.cos(), heading.sin());
        let mut last = probe(from[0], from[1]).map_or(f32::NAN, |s| s.height);
        let mut d = RUN_STEP;
        while d <= limit {
            match probe(from[0] + dx * d, from[1] + dy * d) {
                Some(s) if s.road && (s.height - last).abs() <= MAX_RISE => last = s.height,
                _ => break,
            }
            d += RUN_STEP;
        }
        d - RUN_STEP
    };
    let heading = (0..HEADINGS)
        .map(|i| std::f32::consts::TAU * i as f32 / HEADINGS as f32)
        .max_by(|&a, &b| run(at, a, RUN_LENGTH).total_cmp(&run(at, b, RUN_LENGTH)))
        .unwrap_or(0.0);
    // Centre across the road: half the difference of the free widths to the left and the right.
    let left = heading + std::f32::consts::FRAC_PI_2;
    let to_left = run(at, left, HALF_WIDTH_PROBE);
    let to_right = run(at, left + std::f32::consts::PI, HALF_WIDTH_PROBE);
    let shift = (to_left - to_right) * 0.5;
    let centre = [at[0] + left.cos() * shift, at[1] + left.sin() * shift];
    let height = probe(centre[0], centre[1]).or_else(|| probe(at[0], at[1])).map_or(0.0, |s| s.height);
    Spawn { position: Vec3::new(centre[0], centre[1], height), heading }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An east-west road, 10 m wide, along y = 100 at height 5, with grass around it.
    fn strip(_x: f32, y: f32) -> Option<Surface> {
        let on_road = (y - 100.0).abs() <= 5.0;
        Some(Surface { height: if on_road { 5.0 } else { 4.0 }, road: on_road })
    }

    #[test]
    fn finds_the_road_and_its_direction_and_centre() {
        let spawn = find([20.0, 70.0], strip).expect("a road is 30 m away");
        // East or west, whichever run is longer (they are equal here); never across the road.
        assert!(spawn.heading.sin().abs() < 0.3, "heading {}", spawn.heading);
        assert!((spawn.position.y - 100.0).abs() < 1.5, "centred on the road: y = {}", spawn.position.y);
        assert!((spawn.position.z - 5.0).abs() < 1e-5);
    }

    #[test]
    fn a_start_on_the_road_stays_close() {
        let spawn = find([0.0, 103.0], strip).unwrap();
        assert!((spawn.position.y - 100.0).abs() < 1.5);
        assert!(spawn.position.x.abs() < 1.0, "does not wander along the road");
    }

    #[test]
    fn no_road_no_spawn() {
        assert_eq!(find([0.0, 0.0], |_, _| Some(Surface { height: 0.0, road: false })), None);
        assert_eq!(find([0.0, 0.0], |_, _| None), None);
    }

    #[test]
    fn kerbs_end_a_run() {
        // A road that steps up 2 m at x = 10: the run east of x = 0 stops there, the run west is long.
        let stepped = |x: f32, y: f32| {
            let road = (y - 100.0).abs() <= 5.0;
            Some(Surface { height: if x > 10.0 { 7.0 } else { 5.0 }, road })
        };
        let spawn = find([0.0, 100.0], stepped).unwrap();
        assert!(spawn.heading.cos() < -0.9, "heads west, away from the wall: {}", spawn.heading);
    }
}
