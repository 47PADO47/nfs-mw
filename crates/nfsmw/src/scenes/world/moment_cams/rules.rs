//! When the game moment cameras start: the crash test, the jump test's thresholds and the shot lengths. Spec:
//! `docs/specs/game-moment-camera.md`. Positions are render space (x forward, y left, z up).

use glam::Vec3;

/// The crash camera looks at the car ahead when the player's car is faster than this (m/s)...
pub const CRASH_SPEED: f32 = 20.0;
/// ...or faster than this with a wheel on the ground.
pub const CRASH_SPEED_GROUNDED: f32 = 10.0;
/// The car ahead must be nearer than the distance the player's car covers in this long (s)...
const CRASH_TIME: f32 = 0.25;
/// ...and farther than this (m).
const CRASH_NEAREST: f32 = 3.0;
/// Its direction from the player's car must be within this cosine of the heading...
const CRASH_COSINE: f32 = 0.7;
/// ...and it must stand less than this far off the line of travel (m).
const CRASH_SIDE: f32 = 2.75;
/// A crash shot lasts this long (s).
pub const CRASH_SECS: f32 = 2.0;

/// The pursuit camera's clock starts at this (s); it looks at the cop while the clock is between 0 and
/// [`COP_SHOT_FROM`], so the shot starts half a second after the pursuit and lasts two seconds.
pub const COP_CLOCK: f32 = 2.5;
pub const COP_SHOT_FROM: f32 = 2.0;

/// Whether the player's car is about to hit one of `cars` (render space positions of the other active cars): faster
/// than [`CRASH_SPEED`] (or [`CRASH_SPEED_GROUNDED`] with a wheel down), with a car between 3 m and a quarter second
/// ahead, nearly straight ahead and less than 2.75 m off the line.
pub fn crash_ahead(player: Vec3, forward: Vec3, speed: f32, grounded: bool, cars: &[Vec3]) -> bool {
    let fast = speed > CRASH_SPEED || (grounded && speed > CRASH_SPEED_GROUNDED);
    if !fast {
        return false;
    }
    let reach = CRASH_TIME * speed;
    cars.iter().any(|&car| {
        let to = car - player;
        let distance = to.length();
        if distance <= CRASH_NEAREST || distance >= reach {
            return false;
        }
        let cosine = (to / distance).dot(forward);
        if cosine <= CRASH_COSINE {
            return false;
        }
        let sine = (1.0 - cosine.clamp(0.0, 1.0).powi(2)).sqrt();
        Vec3::new(to.x, to.y, 0.0).length() * sine < CRASH_SIDE
    })
}

/// How long the jump shot lasts for a predicted airtime (s): twice the airtime, 1 to 4 s, times four thirds.
pub fn jump_secs(airtime: f32) -> f32 {
    (2.0 * airtime).clamp(1.0, 4.0) * (4.0 / 3.0)
}

/// What the jump prediction says about a takeoff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flight {
    /// The mean height over the ground of the predicted path (m).
    pub mean_height: f32,
    /// The greatest height over the ground (m).
    pub peak: f32,
    /// The time until the path meets the ground (s).
    pub airtime: f32,
}

/// The jump shot's thresholds: in a jump zone of the track they are lower.
const PEAK_OPEN: f32 = 2.5;
const PEAK_ZONE: f32 = 1.8;
const LENGTH_OPEN: f32 = 25.0;
const LENGTH_ZONE: f32 = 15.0;

/// Whether a predicted flight is worth the jump shot at this speed (m/s), `zone` being a jump zone of the track: the
/// path stays under 20.1 m and lands within 3.1 s, flies higher than 1 m on average and higher than 2.5 m (1.8 m in a
/// zone) at its peak, and covers more than 25 m (15 m) of ground.
pub fn jump_worth_a_shot(flight: Flight, speed: f32, zone: bool) -> bool {
    let (peak, length) = match zone {
        true => (PEAK_ZONE, LENGTH_ZONE),
        false => (PEAK_OPEN, LENGTH_OPEN),
    };
    flight.mean_height < 20.1
        && flight.peak < 20.1
        && flight.airtime < 3.1
        && flight.mean_height > 1.0
        && flight.peak > peak
        && flight.airtime * speed > length
}

/// The slowest takeoff the jump camera considers (m/s).
pub const JUMP_SPEED: f32 = 10.0;

#[cfg(test)]
mod tests {
    use super::*;

    const AHEAD: Vec3 = Vec3::X;

    #[test]
    fn a_car_a_quarter_second_ahead_on_the_line_is_a_crash() {
        let car = [Vec3::new(6.0, 0.5, 0.0)];
        assert!(crash_ahead(Vec3::ZERO, AHEAD, 30.0, true, &car), "6 m ahead at 30 m/s is 0.2 s away");
        assert!(!crash_ahead(Vec3::ZERO, AHEAD, 20.0, true, &[Vec3::new(6.0, 0.0, 0.0)]), "5 m reach at 20 m/s");
        assert!(!crash_ahead(Vec3::ZERO, AHEAD, 30.0, true, &[Vec3::new(2.0, 0.0, 0.0)]), "too close");
        assert!(!crash_ahead(Vec3::ZERO, AHEAD, 30.0, true, &[Vec3::new(6.0, 3.0, 0.0)]), "beside the line");
        assert!(!crash_ahead(Vec3::ZERO, AHEAD, 30.0, true, &[Vec3::new(-6.0, 0.0, 0.0)]), "behind");
    }

    #[test]
    fn the_crash_speed_is_lower_with_a_wheel_down() {
        let car = [Vec3::new(3.5, 0.0, 0.0)];
        assert!(crash_ahead(Vec3::ZERO, AHEAD, 15.0, true, &car));
        assert!(!crash_ahead(Vec3::ZERO, AHEAD, 15.0, false, &car), "airborne needs more than 20 m/s");
        assert!(!crash_ahead(Vec3::ZERO, AHEAD, 9.0, true, &car));
    }

    #[test]
    fn the_jump_shot_lasts_twice_the_airtime_within_its_limits() {
        assert!((jump_secs(0.2) - 4.0 / 3.0).abs() < 1e-5);
        assert!((jump_secs(1.0) - 8.0 / 3.0).abs() < 1e-5);
        assert!((jump_secs(3.0) - 16.0 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn a_jump_needs_height_and_length_and_less_in_a_zone() {
        let flight = Flight { mean_height: 1.5, peak: 2.0, airtime: 1.0 };
        assert!(!jump_worth_a_shot(flight, 30.0, false), "2 m peak is too low in the open");
        assert!(jump_worth_a_shot(flight, 30.0, true), "but enough in a jump zone");
        let big = Flight { mean_height: 3.0, peak: 5.0, airtime: 1.2 };
        assert!(jump_worth_a_shot(big, 30.0, false));
        assert!(!jump_worth_a_shot(big, 15.0, false), "18 m of ground is too short in the open");
        assert!(!jump_worth_a_shot(Flight { airtime: 3.5, ..big }, 30.0, false), "too long a flight");
        assert!(!jump_worth_a_shot(Flight { mean_height: 0.5, ..big }, 30.0, false), "a skim");
    }
}
