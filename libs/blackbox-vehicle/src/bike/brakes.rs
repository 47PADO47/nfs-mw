//! The brakes: the linked levers, the torque at each wheel and the anti-lock multiplier (`docs/specs/ride6-engine-drivetrain.md`
//! "Joined brakes", `docs/specs/ride6-rider-help.md` "ABS").

use super::choices;
use super::curve::ramp;
use super::spec::{AbsSpec, BrakeSpec};

/// The two lever inputs after the linking, 0 to 1.
pub fn joined(spec: &BrakeSpec, front: f32, rear: f32, moving: bool) -> (f32, f32) {
    let (front, rear) = (front.clamp(0.0, 1.0), rear.clamp(0.0, 1.0));
    if !moving {
        return (front, rear);
    }
    if spec.joined_front <= 0.0 && spec.joined_rear <= 0.0 {
        return (front, rear.max(front * spec.joined_factor));
    }
    let larger = front.max(rear);
    (spec.joined_front * larger, spec.joined_rear * larger)
}

/// The torque a brake puts on a wheel of radius `radius` (N m) at lever `lever`: the brake's force at the tyre patch times the
/// radius (the whole brake force of the bike is the front's, the rear brake is the hand-brake channel with its own force).
/// `abs` is the multiplier of the anti-lock system.
pub fn torque(spec: &BrakeSpec, front: bool, lever: f32, abs: f32, radius: f32) -> f32 {
    let force = match front {
        true => spec.front_force,
        false => spec.rear_force,
    };
    lever.clamp(0.0, 1.0) * abs.clamp(0.0, 1.0) * force * radius
}

/// The anti-lock state of the two wheels: how much of the brake each has been released, 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AntiLock {
    front: f32,
    rear: f32,
}

impl AntiLock {
    /// Bring the releases up to date. `normalised_*` is the wheel's longitudinal slip normalised to the force peak (-1 at the
    /// braking peak), as the tyre reports it; a wheel in the air passes 0.
    pub fn update(&mut self, spec: &AbsSpec, speed: f32, normalised_front: f32, normalised_rear: f32, dt: f32) {
        self.update_at(spec, speed, [normalised_front, normalised_rear], choices::ABS_FALL_RATE, dt);
    }

    /// [`AntiLock::update`] with a given fall rate (1/s). The reference filters each release with a rise rate of 10000 and the
    /// fall rate `ABSFallRate` (also 10000 for every bike): up at once, down by the rate times the step, never past the target
    /// (`ride6-powertrain-round6.md`, section 5).
    pub fn update_at(
        &mut self,
        spec: &AbsSpec,
        speed: f32,
        [normalised_front, normalised_rear]: [f32; 2],
        fall_rate: f32,
        dt: f32,
    ) {
        let fade = ramp(speed.abs(), spec.speed_range);
        let target = |intensity: f32, slip: [f32; 2], normalised: f32| intensity * fade * ramp(normalised, slip);
        let fall = fall_rate * dt;
        let front = target(spec.front_intensity, spec.front_slip, normalised_front);
        let rear = target(spec.rear_intensity, spec.rear_slip, normalised_rear);
        self.front = front.max(self.front - fall);
        self.rear = rear.max(self.rear - fall);
    }

    /// The multipliers of the front and rear brake torque.
    pub fn multipliers(&self) -> (f32, f32) {
        (1.0 - self.front, 1.0 - self.rear)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    #[test]
    fn the_front_lever_brings_the_rear_brake_with_it_when_moving() {
        let spec = BikeSpec::example().brakes;
        assert_eq!(joined(&spec, 0.8, 0.0, true), (0.8, 0.8));
        assert_eq!(joined(&spec, 0.8, 0.0, false), (0.8, 0.0));
        assert_eq!(joined(&spec, 0.2, 0.6, true), (0.2, 0.6));
    }

    #[test]
    fn the_shares_replace_the_factor_when_they_are_stated() {
        let spec = BrakeSpec { joined_front: 0.7, joined_rear: 0.3, ..BikeSpec::example().brakes };
        let (front, rear) = joined(&spec, 0.0, 1.0, true);
        assert!((front - 0.7).abs() < 1e-6 && (rear - 0.3).abs() < 1e-6);
    }

    #[test]
    fn each_brake_gives_its_force_at_the_patch() {
        let spec = BikeSpec::example().brakes;
        let r = 0.32;
        assert!((torque(&spec, true, 1.0, 1.0, r) / r - spec.front_force).abs() < 1e-2);
        assert!((torque(&spec, false, 1.0, 1.0, r) / r - spec.rear_force).abs() < 1e-2);
        assert_eq!(torque(&spec, true, 0.5, 0.0, r), 0.0);
    }

    #[test]
    fn anti_lock_releases_the_brake_near_the_peak_and_lets_go_at_the_fall_rate() {
        let spec = BikeSpec::example().brakes.abs;
        let mut abs = AntiLock::default();
        let dt = 1.0 / 60.0;
        abs.update(&spec, 20.0, -1.0, 0.0, dt);
        assert_eq!(abs.multipliers(), (0.0, 1.0));
        // At a slow fall rate the release lingers; the rate is in releases per second.
        abs.update_at(&spec, 20.0, [0.0, 0.0], 6.0, dt);
        let (front, _) = abs.multipliers();
        assert!((front - 0.1).abs() < 1e-5, "{front}");
    }

    #[test]
    fn the_fall_rate_of_the_reference_lets_go_in_a_single_step() {
        let spec = BikeSpec::example().brakes.abs;
        let mut abs = AntiLock::default();
        abs.update(&spec, 20.0, -1.0, -1.0, 1.0 / 60.0);
        assert_eq!(abs.multipliers(), (0.0, 0.0));
        abs.update(&spec, 20.0, 0.0, 0.0, 1.0 / 60.0);
        assert_eq!(abs.multipliers(), (1.0, 1.0));
    }

    #[test]
    fn anti_lock_is_off_at_a_crawl_and_below_the_slip_start() {
        let spec = BikeSpec::example().brakes.abs;
        let mut abs = AntiLock::default();
        abs.update(&spec, 1.0, -1.0, -1.0, 1.0 / 60.0);
        assert_eq!(abs.multipliers(), (1.0, 1.0));
        abs.update(&spec, 20.0, -0.1, -0.1, 1.0 / 60.0);
        assert_eq!(abs.multipliers(), (1.0, 1.0));
    }
}
