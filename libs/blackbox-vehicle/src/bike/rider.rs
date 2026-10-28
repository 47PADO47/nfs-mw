//! The rider's body as a mass: where it sits, how its weight shifts, and the combined centre of gravity
//! (`docs/specs/ride6-rider-body-aero.md`).

use glam::Vec3;

use super::spec::ChassisSpec;

/// The bike's own centre of gravity in the body frame (the origin is on the ground midway between the contact patches).
pub fn bike_cog(chassis: &ChassisSpec) -> Vec3 {
    Vec3::new(0.0, chassis.cg_height, chassis.wheelbase * (chassis.front_weight - 0.5))
}

/// Where the rider's centre of mass is in the body frame when the weight input (filtered) is `weight`: -1 fully back to 1
/// fully forward.
pub fn rider_position(chassis: &ChassisSpec, weight: f32) -> Vec3 {
    let shift = match weight >= 0.0 {
        true => chassis.rider_forward_shift * weight.min(1.0),
        false => chassis.rider_backward_shift * (-weight).min(1.0),
    };
    bike_cog(chassis) + chassis.rider_offset + shift
}

/// The centre of gravity of bike and rider together (body frame): the two positions weighted by mass.
pub fn combined_cog(chassis: &ChassisSpec, weight: f32) -> Vec3 {
    let total = (chassis.mass + chassis.rider_mass).max(f32::EPSILON);
    (bike_cog(chassis) * chassis.mass + rider_position(chassis, weight) * chassis.rider_mass) / total
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
        let cog = combined_cog(&c, 0.0);
        let bike = bike_cog(&c);
        assert!(cog.y > bike.y && cog.y < bike.y + c.rider_offset.y);
        let share = c.rider_mass / (c.mass + c.rider_mass);
        assert!((cog.y - (bike.y + share * c.rider_offset.y)).abs() < 1e-6);
    }

    #[test]
    fn a_rider_with_no_mass_leaves_the_bike_where_it_is() {
        let mut c = BikeSpec::example().chassis;
        c.rider_mass = 0.0;
        assert_eq!(combined_cog(&c, 1.0), bike_cog(&c));
    }

    #[test]
    fn shifting_the_weight_forwards_moves_the_centre_of_gravity_forwards() {
        let c = BikeSpec::example().chassis;
        assert!(combined_cog(&c, 1.0).z > combined_cog(&c, 0.0).z);
        assert!(combined_cog(&c, -1.0).z < combined_cog(&c, 0.0).z);
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
}
