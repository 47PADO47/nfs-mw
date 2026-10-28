//! The rider's body as a mass: where it sits, how its weight shifts, and the combined centre of gravity
//! (`docs/specs/ride6-rider-body-aero.md`).

use glam::Vec3;

use super::curve::ramp;
use super::rider_spec::RiderSpec;
use super::spec::ChassisSpec;

/// The bike's own centre of gravity in the body frame (the origin is on the ground midway between the contact patches).
pub fn bike_cog(chassis: &ChassisSpec) -> Vec3 {
    Vec3::new(0.0, chassis.cg_height, chassis.wheelbase * (chassis.front_weight - 0.5))
}

/// Where the rider's centre of mass is in the body frame when his place is `offset` from the rest place (the shift the
/// weight, the lean and the speed ask for, [`super::rider_place::place`]).
pub fn rider_position(chassis: &ChassisSpec, offset: Vec3) -> Vec3 {
    bike_cog(chassis) + chassis.rider_offset + offset
}

/// The centre of gravity of bike and rider together (body frame): the two positions weighted by mass. `offset` is the rider's
/// asked-for place and `deflection` how far the rider's spring has let him move from it (body frame, m).
pub fn combined_cog(chassis: &ChassisSpec, offset: Vec3, deflection: Vec3) -> Vec3 {
    let total = (chassis.mass + chassis.rider_mass).max(f32::EPSILON);
    (bike_cog(chassis) * chassis.mass + (rider_position(chassis, offset) + deflection) * chassis.rider_mass) / total
}

/// The most the rider's spring may let him move (m): a guard against a collision's impulse.
const MAX_DEFLECTION: f32 = 0.3;

/// The rider as a point mass on a spring and damper (`ride6-rider-body-aero.md`): it lags the bike's acceleration.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RiderBody {
    pub deflection: Vec3,
    speed: Vec3,
}

impl RiderBody {
    /// Advance by `dt` under the bike's acceleration `acceleration` (body frame, m/s^2): the rider is pushed back by it, and
    /// the spring pulls him home. Pass the specific force (acceleration less gravity) for a rider who sags on his spring, the
    /// plain acceleration for one whose rest place is raised to cancel the sag (`correct_gravity`). The damper is scaled by
    /// the speed (`damp_multiplier` at the low end of the range, 1 at the high end) and applied implicitly.
    pub fn step(&mut self, chassis: &ChassisSpec, rider: &RiderSpec, acceleration: Vec3, speed: f32, dt: f32) {
        if chassis.rider_k <= 0.0 || chassis.rider_mass <= 0.0 || dt <= 0.0 {
            *self = Self::default();
            return;
        }
        let mass = chassis.rider_mass;
        let blend = ramp(speed.abs(), rider.damp_speed_range);
        let damp = chassis.rider_damp * (rider.damp_multiplier + (1.0 - rider.damp_multiplier) * blend);
        let pull = -self.deflection * (chassis.rider_k / mass) - acceleration;
        self.speed = (self.speed + pull * dt) / (1.0 + dt * damp / mass);
        self.deflection =
            (self.deflection + self.speed * dt).clamp(Vec3::splat(-MAX_DEFLECTION), Vec3::splat(MAX_DEFLECTION));
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The weight input of the rider after the rate limit of the input channel. The weight filters of the data have a stiffness of
/// 0 and pass their input through, so the rate limit is all that slows the weight (`ride6-rider-body-aero.md`).
#[derive(Clone, Debug, Default)]
pub struct RiderWeight {
    value: f32,
}

impl RiderWeight {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    /// Move the weight towards the input (-1 back to 1 forward) by at most `rate` per second (at once for a rate of 0);
    /// returns it.
    pub fn step(&mut self, input: f32, rate: f32, dt: f32) -> f32 {
        let target = input.clamp(-1.0, 1.0);
        self.value = match rate > 0.0 {
            true => self.value + (target - self.value).clamp(-rate * dt, rate * dt),
            false => target,
        };
        self.value
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    #[test]
    fn the_combined_centre_of_gravity_lies_between_the_bike_and_the_rider() {
        let c = BikeSpec::example().chassis;
        let cog = combined_cog(&c, Vec3::ZERO, Vec3::ZERO);
        let bike = bike_cog(&c);
        assert!(cog.y > bike.y && cog.y < bike.y + c.rider_offset.y);
        let share = c.rider_mass / (c.mass + c.rider_mass);
        assert!((cog.y - (bike.y + share * c.rider_offset.y)).abs() < 1e-6);
    }

    #[test]
    fn a_rider_with_no_mass_leaves_the_bike_where_it_is() {
        let mut c = BikeSpec::example().chassis;
        c.rider_mass = 0.0;
        assert_eq!(combined_cog(&c, Vec3::new(0.3, 0.2, 0.1), Vec3::ZERO), bike_cog(&c));
    }

    #[test]
    fn shifting_the_rider_moves_the_centre_of_gravity_by_his_share_of_the_mass() {
        let c = BikeSpec::example().chassis;
        let rest = combined_cog(&c, Vec3::ZERO, Vec3::ZERO);
        let share = c.rider_mass / (c.mass + c.rider_mass);
        let moved = combined_cog(&c, Vec3::new(0.12, -0.03, 0.1), Vec3::ZERO);
        assert!((moved - rest).distance(Vec3::new(0.12, -0.03, 0.1) * share) < 1e-6);
        // A deflection of his spring counts the same way.
        let sagged = combined_cog(&c, Vec3::ZERO, Vec3::new(0.0, -0.03, 0.0));
        assert!((sagged.y - (rest.y - 0.03 * share)).abs() < 1e-6);
    }

    #[test]
    fn the_weight_moves_at_the_rate_of_the_input_channel() {
        let mut w = RiderWeight::new();
        w.step(1.0, 6.0, 0.1);
        assert!((w.value() - 0.6).abs() < 1e-6);
        for _ in 0..10 {
            w.step(1.0, 6.0, 0.1);
        }
        assert_eq!(w.value(), 1.0);
        assert_eq!(RiderWeight::new().step(-1.0, 0.0, 0.1), -1.0);
    }

    #[test]
    fn a_rider_on_a_spring_lags_the_acceleration_and_comes_home() {
        let c = BikeSpec::example().chassis;
        let spec = RiderSpec::default();
        let mut rider = RiderBody::default();
        // Accelerating forwards pushes the rider back; a settled acceleration gives a steady lag of m a / k.
        for _ in 0..600 {
            rider.step(&c, &spec, Vec3::new(0.0, 0.0, 5.0), 10.0, 1.0 / 60.0);
        }
        let lag = -5.0 * c.rider_mass / c.rider_k;
        assert!((rider.deflection.z - lag).abs() < 1e-3, "{} {lag}", rider.deflection.z);
        for _ in 0..600 {
            rider.step(&c, &spec, Vec3::ZERO, 10.0, 1.0 / 60.0);
        }
        assert!(rider.deflection.length() < 1e-3);
        let mut rigid = c;
        rigid.rider_k = 0.0;
        rider.step(&rigid, &spec, Vec3::new(0.0, 0.0, 50.0), 10.0, 1.0 / 60.0);
        assert_eq!(rider.deflection, Vec3::ZERO);
    }

    #[test]
    fn the_damper_stiffens_with_speed_from_its_multiplier_to_one() {
        let c = BikeSpec::example().chassis;
        let soft = RiderSpec { damp_multiplier: 0.1, ..RiderSpec::default() };
        let after = |speed: f32| {
            let mut rider = RiderBody::default();
            rider.step(&c, &soft, Vec3::new(0.0, 0.0, 20.0), speed, 0.01);
            rider.step(&c, &soft, Vec3::new(0.0, 0.0, 20.0), speed, 0.01);
            rider.deflection.z.abs()
        };
        // Less damping lets him move further in the same time; the range is 40 to 60 km/h.
        assert!(after(5.0) > after(20.0));
        assert!((after(60.0 / 3.6) - after(100.0)).abs() < 1e-9, "from 60 km/h the multiplier is 1");
        assert!(after(10.0) == after(0.0), "below 40 km/h it is the multiplier");
    }

    #[test]
    fn the_sag_under_his_weight_is_there_unless_the_rest_place_is_corrected() {
        let c = BikeSpec::example().chassis;
        let spec = RiderSpec::default();
        let mut sags = RiderBody::default();
        let mut held = RiderBody::default();
        for _ in 0..1200 {
            // At rest the specific force is gravity upward; the corrected rest place feels no acceleration at all.
            sags.step(&c, &spec, Vec3::Y * 9.81, 0.0, 1.0 / 60.0);
            held.step(&c, &spec, Vec3::ZERO, 0.0, 1.0 / 60.0);
        }
        let sag = -9.81 * c.rider_mass / c.rider_k;
        assert!((sags.deflection.y - sag).abs() < 1e-3, "{}", sags.deflection.y);
        assert!(held.deflection.length() < 1e-6);
    }
}
