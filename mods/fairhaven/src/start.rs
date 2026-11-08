//! Where the player starts in the world. The 2012 data has no start of its own (the game starts events and the open city from
//! its gameplay data, which is not read), so the start is found once, when the manifest is made: on the largest flat piece of road
//! of the unit that has the most road, facing along its longest edge.

use anyhow::Result;

use crate::collision::{self, Face, POLY_SOUP_LIST};
use crate::install::Install;
use crate::space;

/// The face flag word's first bit marks roads and other ground a car drives on (`collision::surface_of`).
const DRIVE_BIT: u32 = 1;
/// How flat a face is to be a start: the cosine of its tilt.
const FLAT: f32 = 0.98;
/// How far above the road the start is given, metres.
const LIFT: f32 = 1.0;

/// A start in render space, heading in degrees from +x counter-clockwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Start {
    pub position: [f32; 3],
    pub heading: f32,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(u: [f32; 3], v: [f32; 3]) -> [f32; 3] {
    [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// The area of the flat road in `faces` (engine space), square metres, and the largest such face.
pub fn road(faces: &[Face]) -> (f32, Option<&Face>) {
    let mut total = 0.0;
    let mut best: Option<(f32, &Face)> = None;
    for face in faces.iter().filter(|face| face.flags & DRIVE_BIT != 0) {
        let normal = cross(sub(face.points[1], face.points[0]), sub(face.points[2], face.points[0]));
        let doubled_area = length(normal);
        if doubled_area <= 0.0 || normal[1].abs() / doubled_area < FLAT {
            continue;
        }
        total += doubled_area / 2.0;
        if best.is_none_or(|(area, _)| doubled_area > area) {
            best = Some((doubled_area, face));
        }
    }
    (total, best.map(|(_, face)| face))
}

/// The start on the largest flat road face of `faces` (engine space), if there is one.
pub fn from_faces(faces: &[Face]) -> Option<Start> {
    let face = road(faces).1?;
    let centre = [0, 1, 2].map(|axis| (face.points[0][axis] + face.points[1][axis] + face.points[2][axis]) / 3.0);
    let edges = [(0, 1), (1, 2), (2, 0)].map(|(a, b)| sub(face.points[b], face.points[a]));
    let longest = edges.into_iter().max_by(|a, b| length(*a).total_cmp(&length(*b)))?;
    let [x, y, _] = space::render(longest);
    let heading = y.atan2(x).to_degrees();
    let [px, py, pz] = space::render(centre);
    Some(Start { position: [px, py, pz + LIFT], heading })
}

/// The start of the world of `install`: on the largest flat road face of the unit with the most flat road (the units hold
/// the streets of a district each, so this is the middle of the busiest part of the city).
pub fn choose(install: &Install) -> Result<Option<Start>> {
    let mut best: Option<(f32, Start)> = None;
    for zone in install.zones() {
        let unit = install.open_unit(zone.unit)?;
        let mut faces = Vec::new();
        for id in unit.ids_of(POLY_SOUP_LIST) {
            faces.extend(collision::decode(&unit.resource(id)?.blocks[0])?);
        }
        let (area, _) = road(&faces);
        let Some(start) = from_faces(&faces) else { continue };
        if best.is_none_or(|(most, _)| area > most) {
            best = Some((area, start));
        }
    }
    Ok(best.map(|(_, start)| start))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(points: [[f32; 3]; 3], flags: u32) -> Face {
        Face { points, flags }
    }

    #[test]
    fn the_start_is_on_the_largest_flat_road_face_facing_along_its_longest_edge() {
        let small = face([[0.0, 5.0, 0.0], [2.0, 5.0, 0.0], [0.0, 5.0, 2.0]], DRIVE_BIT);
        // Engine space: x left, y up, z forward. A long road face 4 m wide along z, 40 m long.
        let road = face([[100.0, 10.0, 0.0], [104.0, 10.0, 0.0], [100.0, 10.0, 40.0]], DRIVE_BIT);
        let wall = face([[0.0, 0.0, 0.0], [500.0, 0.0, 0.0], [0.0, 500.0, 0.0]], DRIVE_BIT);
        let off_road = face([[0.0, 0.0, 0.0], [500.0, 0.0, 0.0], [0.0, 0.0, 500.0]], 0x40);
        let start = from_faces(&[small, wall, off_road, road]).unwrap();
        // The centroid (101.33, 10, 13.33) in render space is (13.33, 101.33, 10), lifted by a metre.
        assert!((start.position[0] - 13.333).abs() < 0.01, "{:?}", start.position);
        assert!((start.position[1] - 101.333).abs() < 0.01);
        assert!((start.position[2] - 11.0).abs() < 1e-4);
        // The longest edge runs from (104, 0) to (100, 40) in engine x, z: render (40, -4): a heading of about -5.7 degrees.
        assert!((start.heading - (-4.0f32).atan2(40.0).to_degrees()).abs() < 0.01, "{}", start.heading);
    }

    #[test]
    fn the_flat_road_area_adds_up_and_leaves_out_walls_and_ground() {
        let road_face = face([[0.0, 1.0, 0.0], [10.0, 1.0, 0.0], [0.0, 1.0, 10.0]], DRIVE_BIT);
        let wall = face([[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [0.0, 5.0, 0.0]], DRIVE_BIT);
        let ground = face([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 0.0, 10.0]], 0x40);
        let (area, best) = road(&[road_face, wall, ground, road_face]);
        assert_eq!(area, 100.0, "two road faces of 50 square metres");
        assert_eq!(best, Some(&road_face));
        assert_eq!(road(&[wall, ground]), (0.0, None));
    }

    #[test]
    fn no_flat_road_face_gives_no_start() {
        assert_eq!(from_faces(&[]), None);
        let wall = face([[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [0.0, 5.0, 0.0]], DRIVE_BIT);
        assert_eq!(from_faces(&[wall]), None);
    }
}
