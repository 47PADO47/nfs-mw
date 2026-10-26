//! The markers a converted car carries, derived from its own body.
//!
//! The game starts the nitrous flames at the markers `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the body solid. The 2012 files
//! have locators for the tail pipes, but their table is not decoded (`docs/formats/chameleon-customization.md`), so the
//! pipes are put where tail pipes sit on a car of this body: at the rear face, low, a little inside the corners. This is
//! an approximation made from the car's own shape; it is not the 2012 game's data.
//!
//! The 2012 body has no mount for an aftermarket spoiler or roof scoop (the game has none to offer), so none is made.
//!
//! The lamp markers (`LEFT_HEADLIGHT`, `LEFT_BRAKELIGHT` and their relatives) are not derived from the body: they are the
//! 2012 car's own lamp locators, picked by the lamp records that use them (`docs/formats/chameleon-lamps.md`).

use blackbox_hash::bstring_hash;
use blackbox_solid::PositionMarker;
use chameleon_vehicle::{LampEntry, Lamps};
/// How far from the rear face a point still counts as the rear when the car's width there is measured, in metres.
const REAR_DEPTH: f32 = 0.4;
/// The pipes' lateral place as a fraction of the half width of the rear.
const PIPE_SPREAD: f32 = 0.5;
/// The pipes' height as a fraction of the car's height.
const PIPE_HEIGHT: f32 = 0.22;

/// The tail pipe markers of a body whose points, in car space (+x forward, +y left, +z up), are `body`. Empty for a body
/// with no points.
pub fn tail_pipes(body: &[[f32; 3]]) -> Vec<PositionMarker> {
    let Some(rear) = body.iter().map(|p| p[0]).reduce(f32::min) else { return Vec::new() };
    let (low, high) = body.iter().map(|p| p[2]).fold((f32::MAX, f32::MIN), |(low, high), z| (low.min(z), high.max(z)));
    let half_width = body.iter().filter(|p| p[0] <= rear + REAR_DEPTH).map(|p| p[1].abs()).fold(0.0, f32::max);
    let at = |side: f32| [rear, side * half_width * PIPE_SPREAD, low + PIPE_HEIGHT * (high - low)];
    [("LEFT_EXHAUST", 1.0), ("RIGHT_EXHAUST", -1.0)].into_iter().map(|(name, side)| marker(name, at(side))).collect()
}

/// The most lamps of one kind on one side of the car that get a marker.
const MAX_PER_SIDE: usize = 2;
/// Two lamp places closer than this, metres, are one lamp.
const SAME_PLACE: f32 = 0.02;
/// A lamp this close to the centre plane, metres, is a centre lamp.
const CENTRE_BAND: f32 = 0.05;

/// One lamp: where it is in the game's car space and the way it shines (a unit vector).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Spot {
    at: [f32; 3],
    shines: [f32; 3],
}

/// The lamp markers of a car: the head lamps, the brake lamps and the centre brake lamp, in car space (`to_car` turns a
/// point of the 2012 vehicle space into the game's car space, the way the body was moved).
///
/// A car has no marker of a kind its records do not give (the cars with no lamp records have none at all, and no 2012
/// record is a reverse lamp, so there are no reverse markers).
pub fn lamp_markers(lamps: &Lamps, to_car: &dyn Fn([f32; 3]) -> [f32; 3]) -> Vec<PositionMarker> {
    let mut out = Vec::new();
    let heads = spots(lamps, |entry| matches!(entry.state_bit, 0 | 6) && !is_amber(entry), true, to_car);
    let brakes = match spots(lamps, |entry| entry.state_bit == 1, false, to_car) {
        found if !found.is_empty() => found,
        _ => spots(lamps, |entry| matches!(entry.state_bit, 0 | 7), false, to_car),
    };
    for (spot, name) in sided(&heads, "LEFT_HEADLIGHT", "RIGHT_HEADLIGHT", None) {
        out.push(marker_at(name, spot));
    }
    for (spot, name) in sided(&brakes, "LEFT_BRAKELIGHT", "RIGHT_BRAKELIGHT", Some("BRAKELIGHT_CENTRE")) {
        out.push(marker_at(name, spot));
    }
    out
}

/// A record whose colour factor is orange (a position or turn lamp, not a head lamp).
fn is_amber(entry: &LampEntry) -> bool {
    let [r, g, b, _] = entry.tint;
    let high = r.max(g).max(b);
    high > 0.0 && r.min(g).min(b) < 0.5 * high
}

/// The places of the lamps of the corona records that `wanted` takes, at the front or the rear of the car. Each place and
/// its mirror image count, because a car is symmetrical and the records name one side or both (the mode of a record is
/// not decoded far enough to tell).
fn spots(
    lamps: &Lamps,
    wanted: impl Fn(&LampEntry) -> bool,
    front: bool,
    to_car: &dyn Fn([f32; 3]) -> [f32; 3],
) -> Vec<Spot> {
    let mut found: Vec<Spot> = Vec::new();
    for entry in lamps.coronas.iter().filter(|entry| wanted(entry)) {
        let Some(group) = lamps.group(entry.group) else { continue };
        for locator in &group.locators {
            let p = locator.position;
            if (p[2] > 0.0) != front {
                continue;
            }
            let (turned, origin) = (to_car(locator.forward()), to_car([0.0; 3]));
            let shines = unit([turned[0] - origin[0], turned[1] - origin[1], turned[2] - origin[2]], front);
            let at = to_car(p);
            for side in [1.0, -1.0] {
                let spot = Spot { at: [at[0], side * at[1], at[2]], shines: [shines[0], side * shines[1], shines[2]] };
                let seen = found.iter().any(|known| distance(known.at, spot.at) < SAME_PLACE);
                if !seen {
                    found.push(spot);
                }
            }
        }
    }
    found
}

/// A unit vector along `v`, turned to the end of the car it belongs to when it points the wrong way or is too flat.
fn unit(v: [f32; 3], front: bool) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let end = if front { 1.0 } else { -1.0 };
    if length < 1e-6 || v[0] * end / length < 0.5 {
        return [end, 0.0, 0.0];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The spots with the name of their side, at most [`MAX_PER_SIDE`] a side; centre spots take `centre` (none: left out).
fn sided<'a>(spots: &[Spot], left: &'a str, right: &'a str, centre: Option<&'a str>) -> Vec<(Spot, &'a str)> {
    let mut count = [0, 0];
    let mut out = Vec::new();
    for &spot in spots {
        let y = spot.at[1];
        let (name, slot) = match y {
            y if y > CENTRE_BAND => (left, 0),
            y if y < -CENTRE_BAND => (right, 1),
            _ => {
                let Some(name) = centre else { continue };
                out.push((spot, name));
                continue;
            }
        };
        if count[slot] < MAX_PER_SIDE {
            count[slot] += 1;
            out.push((spot, name));
        }
    }
    out
}

/// A lamp marker: the z axis is the way the lamp shines, the x axis points up, as on the game's own cars.
fn marker_at(name: &str, spot: Spot) -> PositionMarker {
    let z = spot.shines;
    let dot = z[2];
    let up = [-dot * z[0], -dot * z[1], 1.0 - dot * z[2]];
    let length = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
    let x = [up[0] / length, up[1] / length, up[2] / length];
    let y = [z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0]];
    let matrix =
        [x[0], x[1], x[2], 0.0, y[0], y[1], y[2], 0.0, z[0], z[1], z[2], 0.0, spot.at[0], spot.at[1], spot.at[2], 1.0];
    PositionMarker { name_hash: bstring_hash(name), int_param: 0, float_params: [0.0, 0.0], matrix }
}

/// A marker whose z axis points backwards, which is the way a tail pipe blows (the game's flames follow it).
fn marker(name: &str, at: [f32; 3]) -> PositionMarker {
    let mut matrix = [0.0; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[8] = -1.0;
    matrix[15] = 1.0;
    matrix[12..15].copy_from_slice(&at);
    PositionMarker { name_hash: bstring_hash(name), int_param: 0, float_params: [0.0, 0.0], matrix }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box 4 m long, 1.8 m wide, 1.2 m tall, standing on the ground, with its rear at x = -2.
    fn body() -> Vec<[f32; 3]> {
        let mut points = Vec::new();
        for x in [-2.0, 2.0] {
            for y in [-0.9, 0.9] {
                for z in [0.1, 1.3] {
                    points.push([x, y, z]);
                }
            }
        }
        points
    }

    use chameleon_vehicle::{LampEntry, Lamps, Locator, LocatorGroup};

    fn locator(x: f32, y: f32, z: f32, rotation: [f32; 4]) -> Locator {
        Locator { rotation, position: [x, y, z] }
    }

    fn entry(group: u32, state_bit: u16, tint: [f32; 4]) -> LampEntry {
        LampEntry { group, state_bit, mode: 1, tint, level: 1.0 }
    }

    /// A car with one head lamp record at the front right, a brake lamp record at the rear right, a centre brake lamp, an
    /// amber record at the front and a record at the front that is no lamp of ours (bit 3).
    fn lamps() -> Lamps {
        let ahead = [0.0, 0.0, 0.0, 1.0];
        let behind = [0.0, 1.0, 0.0, 0.0];
        Lamps {
            groups: vec![
                LocatorGroup { id: 1, locators: vec![locator(-0.7, 0.65, 2.2, ahead)] },
                LocatorGroup { id: 2, locators: vec![locator(-0.6, 0.75, -2.0, behind)] },
                LocatorGroup { id: 3, locators: vec![locator(0.0, 0.95, -1.8, behind)] },
                LocatorGroup { id: 4, locators: vec![locator(-0.7, 0.7, 1.7, ahead)] },
            ],
            coronas: vec![
                entry(1, 0, [1.0; 4]),
                entry(2, 1, [1.0; 4]),
                entry(3, 1, [1.0; 4]),
                entry(4, 6, [1.0, 0.45, 0.13, 1.0]),
                entry(4, 3, [1.0; 4]),
            ],
            lights: Vec::new(),
        }
    }

    fn find(markers: &[PositionMarker], name: &str) -> Vec<PositionMarker> {
        markers.iter().filter(|m| m.name_hash == bstring_hash(name)).copied().collect()
    }

    #[test]
    fn the_lamp_records_give_head_and_brake_markers_on_both_sides() {
        let markers = lamp_markers(&lamps(), &|p| [p[2], p[0], p[1]]);
        let left = find(&markers, "LEFT_HEADLIGHT");
        let right = find(&markers, "RIGHT_HEADLIGHT");
        assert_eq!((left.len(), right.len()), (1, 1));
        // The record names the right lamp at engine x = -0.7: car y = -0.7. Its mirror is the left lamp.
        assert_eq!(right[0].translation(), [2.2, -0.7, 0.65]);
        assert_eq!(left[0].translation(), [2.2, 0.7, 0.65]);
        assert_eq!(find(&markers, "LEFT_BRAKELIGHT")[0].translation(), [-2.0, 0.6, 0.75]);
        assert_eq!(find(&markers, "RIGHT_BRAKELIGHT")[0].translation(), [-2.0, -0.6, 0.75]);
        assert_eq!(find(&markers, "BRAKELIGHT_CENTRE")[0].translation(), [-1.8, 0.0, 0.95]);
    }

    #[test]
    fn amber_and_other_records_are_no_head_lamps() {
        let markers = lamp_markers(&lamps(), &|p| [p[2], p[0], p[1]]);
        assert_eq!(markers.iter().filter(|m| m.name_hash == bstring_hash("LEFT_HEADLIGHT")).count(), 1);
        assert!(find(&markers, "LEFT_REVERSE").is_empty() && find(&markers, "CENTRE_HEADLIGHT").is_empty());
    }

    #[test]
    fn a_head_lamp_shines_forward_and_a_brake_lamp_backward_with_the_x_axis_up() {
        let markers = lamp_markers(&lamps(), &|p| [p[2], p[0], p[1]]);
        let head = find(&markers, "LEFT_HEADLIGHT")[0];
        let brake = find(&markers, "LEFT_BRAKELIGHT")[0];
        // Row-major as the game's own markers: x axis, y axis, z axis, translation.
        assert_eq!(&head.matrix[8..11], &[1.0, 0.0, 0.0]);
        assert_eq!(&brake.matrix[8..11], &[-1.0, 0.0, 0.0]);
        assert_eq!(&head.matrix[0..3], &[0.0, 0.0, 1.0]);
        assert_eq!(&head.matrix[4..7], &[0.0, -1.0, 0.0]);
        assert_eq!(&brake.matrix[4..7], &[0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_car_without_lamp_records_has_no_lamp_markers() {
        assert!(lamp_markers(&Lamps::default(), &|p| p).is_empty());
    }

    #[test]
    fn a_lamp_with_no_brake_record_falls_back_to_its_tail_record() {
        let mut lamps = lamps();
        lamps.coronas.retain(|c| c.state_bit != 1);
        lamps.coronas.push(entry(2, 7, [1.0; 4]));
        let markers = lamp_markers(&lamps, &|p| [p[2], p[0], p[1]]);
        assert_eq!(find(&markers, "LEFT_BRAKELIGHT").len(), 1);
    }

    #[test]
    fn the_pipes_sit_at_the_rear_face_low_and_either_side_of_the_centre() {
        let pipes = tail_pipes(&body());
        assert_eq!(pipes.len(), 2);
        let [left, right] = [pipes[0].translation(), pipes[1].translation()];
        assert_eq!(left[0], -2.0);
        assert!((left[1] - 0.45).abs() < 1e-5 && (right[1] + 0.45).abs() < 1e-5);
        assert!((left[2] - (0.1 + 0.22 * 1.2)).abs() < 1e-5);
        assert_eq!(pipes[0].name_hash, bstring_hash("LEFT_EXHAUST"));
    }

    #[test]
    fn a_pipe_blows_backwards() {
        let pipe = tail_pipes(&body())[0];
        assert_eq!(&pipe.matrix[8..11], &[-1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_body_without_points_has_no_pipes() {
        assert!(tail_pipes(&[]).is_empty());
    }
}
