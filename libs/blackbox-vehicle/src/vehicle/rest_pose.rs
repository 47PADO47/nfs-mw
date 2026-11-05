//! Where a car set on the ground comes to rest (`docs/specs/vehicle-placement.md`): each wheel's corner of the body box sits
//! at its own ground height plus the axle's ride height, less the spring's static sag under a quarter of the car's weight,
//! and the body's pitch and roll follow the four corners. Placed there with no speed, the springs already hold the car and
//! it does not drop onto them.

use glam::{Mat3, Quat, Vec3};

use crate::ground::Ground;

/// How far above and below the centre's ground a wheel's own ground is searched for, metres.
const WHEEL_SEARCH: f32 = 2.0;
/// Gravity, m/s^2.
const GRAVITY: f32 = 9.81;

/// What the rest pose needs of the car.
pub(super) struct RestCar {
    /// The wheels' points at the bottom of the body box, in the body frame (front left, front right, rear left, rear right).
    pub arms: [Vec3; 4],
    /// Half the body box's height, metres.
    pub half_height: f32,
    pub mass: f32,
    /// The front and rear spring rates, N/m.
    pub springs: [f32; 2],
    /// The front and rear ride heights with the tuning, metres.
    pub rides: [f32; 2],
}

/// The spring's static compression under a quarter of the car's weight, metres: 0 for a spring with no rate.
fn sag(mass: f32, spring: f32) -> f32 {
    if spring <= 0.0 {
        return 0.0;
    }
    mass * GRAVITY * 0.25 / spring
}

/// The body's position and orientation at rest with its centre over `(x, z)`, whose ground is at `ground_y`, facing `yaw`
/// radians about the up axis (0 = +z). A wheel with no ground of its own within two metres of the centre's takes the
/// centre's.
pub(super) fn rest_pose(
    ground: &dyn Ground,
    car: &RestCar,
    (x, ground_y, z): (f32, f32, f32),
    yaw: f32,
) -> (Vec3, Quat) {
    let heading = Quat::from_rotation_y(yaw);
    let centre = Vec3::new(x, ground_y, z);
    let mut points = [Vec3::ZERO; 4];
    let mut flat_mean = Vec3::ZERO;
    for (i, arm) in car.arms.iter().enumerate() {
        let flat = Vec3::new(arm.x, 0.0, arm.z);
        flat_mean += flat * 0.25;
        let at = centre + heading * flat;
        let top = ground_y + WHEEL_SEARCH;
        let face =
            ground.hit(Vec3::new(at.x, top, at.z), Vec3::NEG_Y, WHEEL_SEARCH * 2.0).map(|hit| top - hit.distance);
        let axle = i / 2;
        let height = face.unwrap_or(ground_y) + car.rides[axle] - sag(car.mass, car.springs[axle]) + car.half_height;
        points[i] = Vec3::new(at.x, height, at.z);
    }
    let orientation = fitted(&points).unwrap_or(heading);
    let mean = points.iter().copied().sum::<Vec3>() * 0.25;
    (mean - orientation * flat_mean, orientation)
}

/// The orientation whose forward runs from the rear wheels' midpoint to the front ones' and whose right runs from the left
/// wheels' midpoint to the right ones'; `None` when the points are degenerate.
fn fitted(points: &[Vec3; 4]) -> Option<Quat> {
    let front = (points[0] + points[1]) * 0.5;
    let rear = (points[2] + points[3]) * 0.5;
    let left = (points[0] + points[2]) * 0.5;
    let right = (points[1] + points[3]) * 0.5;
    let forward = (front - rear).try_normalize()?;
    let side = (right - left).try_normalize()?;
    let up = forward.cross(side).try_normalize()?;
    let side = up.cross(forward);
    Some(Quat::from_mat3(&Mat3::from_cols(side, up, forward)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::{FlatGround, SlopedGround, SurfaceGrip};

    fn car() -> RestCar {
        RestCar {
            arms: [
                Vec3::new(-0.8, -0.5, 1.4),
                Vec3::new(0.8, -0.5, 1.4),
                Vec3::new(-0.8, -0.5, -1.2),
                Vec3::new(0.8, -0.5, -1.2),
            ],
            half_height: 0.5,
            mass: 1400.0,
            springs: [40_000.0, 40_000.0],
            rides: [0.12, 0.12],
        }
    }

    #[test]
    fn on_flat_ground_the_car_sits_at_ride_height_less_the_sag_and_level() {
        let (position, orientation) = rest_pose(&FlatGround::new(0.0), &car(), (3.0, 0.0, -2.0), 0.0);
        let sag = 1400.0 * 9.81 * 0.25 / 40_000.0;
        assert!((position.y - (0.12 - sag + 0.5)).abs() < 1e-4, "{position}");
        assert!(
            (position.x - 3.0).abs() < 1e-4,
            "the wheels' centre is not the box's: the box centre comes back over x"
        );
        assert!(orientation.angle_between(Quat::IDENTITY) < 1e-4);
    }

    #[test]
    fn the_heading_is_kept_and_a_softer_rear_sits_lower_and_pitches_the_nose_up() {
        let mut car = car();
        car.springs[1] = 20_000.0;
        let (_, orientation) = rest_pose(&FlatGround::new(0.0), &car, (0.0, 0.0, 0.0), 1.0);
        let forward = orientation * Vec3::Z;
        assert!(forward.y > 0.0, "the rear sags more: {forward}");
        assert!((forward.x.atan2(forward.z) - 1.0).abs() < 1e-3, "the heading stays");
    }

    #[test]
    fn on_a_slope_the_body_pitches_with_the_road() {
        let ground = SlopedGround { height: 0.0, slope: 0.1, surface: SurfaceGrip::DEFAULT };
        let (_, orientation) = rest_pose(&ground, &car(), (0.0, 0.0, 0.0), 0.0);
        let forward = orientation * Vec3::Z;
        assert!((forward.y / forward.z - 0.1).abs() < 1e-3, "the nose follows the 10 % grade: {forward}");
    }

    #[test]
    fn a_spring_without_a_rate_has_no_sag() {
        assert_eq!(sag(1400.0, 0.0), 0.0);
    }
}
