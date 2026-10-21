//! Where a cop may stand when it is put on the road (`docs/specs/ai-pursuit-cops.md` sections 3.1 and 3.2, and the
//! chase spawn of 3.4): the spacing from the cars already there, and the geometry of the walk from the target's
//! seek-ahead point along the cop roads. Pure rules: the caller measures the distances and walks its road graph.

use glam::Vec3;

/// A car in the same lane and heading the same way (or a wreck) keeps this many metres from a new cop (`sqrt(225)`).
pub const SAME_LANE: f32 = 15.0;
/// A car in another lane keeps this many metres (`sqrt(25)`).
pub const OTHER_LANE: f32 = 5.0;
/// A car that does not drive the traffic lanes (a chase cop), or any car when the lane is not checked (a chase spawn),
/// keeps this many metres (`sqrt(1600)`).
pub const ANY_OTHER: f32 = 40.0;

/// How an existing car stands to the lane a patrol cop is about to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neighbour {
    /// It follows the traffic lanes: its lane index and the end of its segment it heads for.
    Traffic { lane: usize, node: usize },
    /// A wreck on the traffic lanes.
    Wreck,
    /// It does not follow the traffic lanes.
    Other,
}

/// The distance a car in the given `neighbour` relation must keep from a cop put in `lane` heading for `node`. With
/// `check_lane` false (a chase spawn) every car keeps [`ANY_OTHER`]. The lane test compares the lane index and the end
/// the cars head for, not the segment, as the original does (the cars close enough to matter are on the same one).
pub fn spacing(check_lane: bool, lane: usize, node: usize, neighbour: Neighbour) -> f32 {
    if !check_lane {
        return ANY_OTHER;
    }
    match neighbour {
        Neighbour::Wreck => SAME_LANE,
        Neighbour::Other => ANY_OTHER,
        Neighbour::Traffic { lane: l, node: n } if l == lane && n == node => SAME_LANE,
        Neighbour::Traffic { .. } => OTHER_LANE,
    }
}

/// Whether a cop put at `lane`/`node` is clear of every `(distance, neighbour)` pair. The distance is the straight
/// distance in metres between the new spot and the car.
pub fn clear(check_lane: bool, lane: usize, node: usize, others: impl IntoIterator<Item = (f32, Neighbour)>) -> bool {
    others.into_iter().all(|(distance, neighbour)| distance >= spacing(check_lane, lane, node, neighbour))
}

/// Seconds of the target's travel the seek-ahead point looks (`kSeekAheadTime`).
pub const SEEK_AHEAD_TIME: f32 = 7.8;
/// Metres added to the distance, and the most it may be.
pub const SEEK_AHEAD_BASE: f32 = 8.0;
pub const SEEK_AHEAD_MAX: f32 = 500.0;

/// How far along its road the target's seek-ahead point lies, metres (`GetSeekAheadPosition`).
pub fn seek_ahead_distance(speed: f32) -> f32 {
    (speed.max(0.0) * SEEK_AHEAD_TIME + SEEK_AHEAD_BASE).min(SEEK_AHEAD_MAX)
}

/// The angle the target-to-seek vector is turned by before the cop walks that far along the roads: 60 degrees.
pub const SIDE_ANGLE: f32 = std::f32::consts::FRAC_PI_3;

/// The turn for a roll in `0..1`: positive when the roll is at most a half, negative above it.
pub fn side_angle(roll: f32) -> f32 {
    match roll > 0.5 {
        true => -SIDE_ANGLE,
        false => SIDE_ANGLE,
    }
}

/// Turns the horizontal part of `v` (x and z; y is up) by `angle` radians about the vertical axis; the height is kept.
pub fn rotate_in_xz(v: Vec3, angle: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    Vec3::new(v.x * cos - v.z * sin, v.y, v.x * sin + v.z * cos)
}

/// Metres ahead of the target the fallback spot is centred, and the radius around it (`kPursuitCarSpawnDistance`,
/// `kPursuitCarSpawnRadius`).
pub const FALLBACK_AHEAD: f32 = 200.0;
pub const FALLBACK_RADIUS: f32 = 190.0;

/// A point of the disc around `centre` of radius [`FALLBACK_RADIUS`], uniform by area, from two rolls in `0..1`.
pub fn disc_point(centre: Vec3, angle_roll: f32, radius_roll: f32) -> Vec3 {
    let angle = std::f32::consts::TAU * angle_roll;
    let radius = radius_roll.clamp(0.0, 1.0).sqrt() * FALLBACK_RADIUS;
    centre + Vec3::new(angle.sin() * radius, 0.0, angle.cos() * radius)
}

/// The fallback spot's centre: [`FALLBACK_AHEAD`] metres ahead of the target along its horizontal velocity, or along its
/// facing when it moves slower than 1 m/s.
pub fn fallback_centre(position: Vec3, velocity: Vec3, facing: Vec3) -> Vec3 {
    let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
    let along = match horizontal.length() < 1.0 {
        true => Vec3::new(facing.x, 0.0, facing.z),
        false => horizontal,
    };
    position + along.normalize_or_zero() * FALLBACK_AHEAD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patrol_keeps_fifteen_metres_in_its_lane_and_five_in_another() {
        let traffic = |lane, node| Neighbour::Traffic { lane, node };
        assert_eq!(spacing(true, 1, 1, traffic(1, 1)), 15.0);
        assert_eq!(spacing(true, 1, 1, traffic(2, 1)), 5.0, "another lane");
        assert_eq!(spacing(true, 1, 1, traffic(1, 0)), 5.0, "the same lane the other way");
        assert_eq!(spacing(true, 1, 1, Neighbour::Wreck), 15.0, "a wreck blocks its lane");
        assert_eq!(spacing(true, 1, 1, Neighbour::Other), 40.0, "a car off the traffic lanes");
    }

    #[test]
    fn a_chase_spawn_keeps_forty_metres_from_everything() {
        for neighbour in [Neighbour::Traffic { lane: 1, node: 1 }, Neighbour::Wreck, Neighbour::Other] {
            assert_eq!(spacing(false, 1, 1, neighbour), 40.0);
        }
    }

    #[test]
    fn a_spot_is_clear_only_when_every_car_keeps_its_distance() {
        let near_other_lane = (6.0, Neighbour::Traffic { lane: 2, node: 1 });
        let near_same_lane = (14.0, Neighbour::Traffic { lane: 1, node: 1 });
        assert!(clear(true, 1, 1, [near_other_lane]));
        assert!(!clear(true, 1, 1, [near_other_lane, near_same_lane]));
        assert!(clear(true, 1, 1, std::iter::empty()));
        assert!(!clear(true, 1, 1, [(4.0, Neighbour::Traffic { lane: 2, node: 1 })]));
    }

    #[test]
    fn the_seek_ahead_point_is_eight_metres_plus_seven_point_eight_seconds_of_travel() {
        assert_eq!(seek_ahead_distance(0.0), 8.0);
        assert!((seek_ahead_distance(30.0) - 242.0).abs() < 1e-3);
        assert_eq!(seek_ahead_distance(200.0), 500.0, "capped");
        assert_eq!(seek_ahead_distance(-5.0), 8.0, "reversing counts as standing");
    }

    #[test]
    fn the_side_is_picked_by_the_roll_and_the_turn_is_sixty_degrees() {
        assert!((side_angle(0.1) - 60f32.to_radians()).abs() < 1e-6);
        assert!((side_angle(0.9) + 60f32.to_radians()).abs() < 1e-6);
        let turned = rotate_in_xz(Vec3::new(0.0, 3.0, 10.0), side_angle(0.0));
        assert_eq!(turned.y, 3.0, "the height stays");
        let horizontal = Vec3::new(turned.x, 0.0, turned.z);
        assert!((horizontal.length() - 10.0).abs() < 1e-4);
        assert!((horizontal.angle_between(Vec3::Z) - 60f32.to_radians()).abs() < 1e-4);
    }

    #[test]
    fn the_fallback_is_centred_two_hundred_metres_ahead_and_stays_in_its_disc() {
        let at = Vec3::new(5.0, 0.0, 5.0);
        let moving = fallback_centre(at, Vec3::new(0.0, 0.0, 20.0), Vec3::X);
        assert!((moving - Vec3::new(5.0, 0.0, 205.0)).length() < 1e-4);
        let standing = fallback_centre(at, Vec3::new(0.0, 0.0, 0.4), Vec3::X);
        assert!((standing - Vec3::new(205.0, 0.0, 5.0)).length() < 1e-4, "slower than 1 m/s: the facing");
        for (a, r) in [(0.0, 0.0), (0.25, 1.0), (0.7, 0.5), (0.99, 0.99)] {
            let p = disc_point(moving, a, r);
            assert!(p.distance(moving) <= FALLBACK_RADIUS + 1e-3);
        }
        assert!((disc_point(moving, 0.0, 1.0) - moving - Vec3::new(0.0, 0.0, 190.0)).length() < 1e-3);
    }
}
