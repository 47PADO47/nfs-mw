//! What the contacts of the game (walls, props, other bodies) do to a bike (`docs/specs/ride6-feel-round10.md`).
//!
//! The contacts act on the body double, a box with the bike's mass and inertia, and what they leave in its velocities the bike takes
//! back (`BikeRide::pull`). A car's body is a box and a wall reaction at a corner of it is a good model; a bike is held up by its
//! wheels, its rider and its lean controller, and a reaction at the corner of the double's box (a metre ahead of the centre of
//! gravity and a hand's width off the heading, lowered by the lean) rolls and pitches it at 3 to 4 rad/s for a glancing touch of the
//! edge of a road, where the angular-speed test of the fall (4.3 to 4.4 rad/s, the data's) and the lean test then fire. So what the
//! bike takes back from a step's contacts is a choice of this port:
//!
//! - **A hard enough contact throws the rider** ([`COLLISION_FALL_SPEED`]): a wall at speed has to crash a bike.
//! - **Below that the bike keeps the change of its velocity and of its yaw, not of its roll and its pitch**: those stay what its own
//!   wheels, suspension and rider make of them. And its controllers help it as the reference's do after a collision
//!   (`Bike::notify_collision`: the lean help at its full factor for 0.4 s, the stability help pulling the yaw rate to the one the
//!   lean needs).

use blackbox_vehicle::Vehicle;
use blackbox_vehicle::bike::FallKind;
use glam::Vec3;

use super::BikeRide;

/// The change of the bike's velocity (m/s) in one step that the contacts of the game cause above which the rider is thrown, whatever
/// the bike's angles: a wall met head-on above 50 km/h, or at 30 degrees above 100 km/h. A modelling choice. The reference has a
/// `FallCollisionSpeedThreshold` of 10 in its eight input handlers, but the data gives no unit or measure for it, and its collision
/// test is limited to the `WorldDynamic` channel (with an impulse limit of 2e6, never): a static wall never trips it there. The sweep
/// of `ride6-feel-round10.md` (section 4): 10 m/s threw 23 of 33 bikes that met a wall at 9 to 12 m/s of closing speed, most of which
/// the controllers' help keeps up; 14 m/s throws every bike that met one at 15 m/s or more and almost none below 9.
pub(super) const COLLISION_FALL_SPEED: f32 = 14.0;
/// A change of velocity smaller than this (m/s) in a step is no collision for the controllers' help: the friction of a wall the bike
/// already follows is not what the help is for. A modelling choice.
const MIN_JOLT: f32 = 0.05;

impl BikeRide {
    /// The bike takes what the contacts of the step did to the double (see the module's text).
    pub(super) fn take_contacts(&mut self, vehicle: &Vehicle) {
        let before = (self.bike.body().linear_velocity, self.bike.body().angular_velocity);
        self.pull(vehicle);
        let change = self.bike.body().linear_velocity - before.0;
        if change.length() > COLLISION_FALL_SPEED {
            self.bike.throw_rider(FallKind::Generic);
            return;
        }
        let rotation = self.bike.body().rotation();
        let body = self.bike.body_mut();
        // Up is +y in physics space: the yaw is the part of the change of the angular velocity along it.
        let yaw = (body.angular_velocity - before.1).dot(Vec3::Y);
        body.angular_velocity = before.1 + Vec3::Y * yaw;
        if change.length() <= MIN_JOLT {
            return;
        }
        let frontal = change.dot(rotation * Vec3::Z).abs() > change.dot(rotation * Vec3::X).abs();
        self.bike.notify_collision(frontal);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use blackbox_vehicle::FlatGround;
    use blackbox_vehicle::bike::BikeSpec;
    use nfsmw_data::car::physics::{BikePhysics, CarBounds, CarPhysics};

    use super::*;

    const BOUNDS: CarBounds =
        CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.05) };

    /// A bike at 15 m/s on a flat road and its double, after a step that the contacts have not touched yet.
    fn riding() -> (BikeRide, Vehicle) {
        let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
        physics.bike = Some(BikePhysics { spec: Arc::new(BikeSpec::example()), origin_x: -0.02 });
        let mut ride = BikeRide::new(&mut physics).expect("a bike");
        let mut vehicle = Vehicle::new(physics.spec);
        ride.attach(&mut vehicle);
        assert!(ride.place(&FlatGround::new(0.0), (0.0, 0.0, 2.0), 0.0, 15.0));
        ride.push(&mut vehicle);
        (ride, vehicle)
    }

    #[test]
    fn a_touch_changes_the_velocity_and_the_yaw_of_the_bike_and_leaves_its_roll_and_pitch() {
        let (mut ride, mut vehicle) = riding();
        let body = vehicle.body_mut();
        body.linear_velocity += Vec3::new(-2.0, 0.0, -0.5);
        body.angular_velocity = Vec3::new(2.0, 1.0, 3.0);
        ride.take_contacts(&vehicle);
        let bike = ride.bike.body();
        assert_eq!(bike.linear_velocity, Vec3::new(-2.0, 0.0, 14.5));
        assert_eq!(bike.angular_velocity, Vec3::new(0.0, 1.0, 0.0));
        assert!(!ride.bike.fallen());
    }

    #[test]
    fn a_touch_raises_the_help_of_the_controllers_and_a_step_without_one_does_not() {
        let (mut ride, mut vehicle) = riding();
        ride.take_contacts(&vehicle);
        assert_eq!(ride.bike.collision_help(), (0.0, 0.0), "nothing touched the double");
        vehicle.body_mut().linear_velocity += Vec3::new(-1.0, 0.0, 0.0);
        ride.take_contacts(&vehicle);
        let (lean, stability) = ride.bike.collision_help();
        assert!(lean > 1.0 && stability > 0.0, "{lean} {stability}");
    }

    #[test]
    fn a_hard_enough_contact_throws_the_rider_and_the_bike_keeps_all_of_it() {
        let (mut ride, mut vehicle) = riding();
        let body = vehicle.body_mut();
        body.linear_velocity += Vec3::new(0.0, 0.0, -(COLLISION_FALL_SPEED + 1.0));
        body.angular_velocity = Vec3::new(2.0, 1.0, 3.0);
        ride.take_contacts(&vehicle);
        assert!(ride.bike.fallen() && ride.bike.rider_off());
        assert!(ride.bike.take_rider_release().is_some(), "the host throws the rider");
        assert_eq!(ride.bike.body().angular_velocity, Vec3::new(2.0, 1.0, 3.0));
    }
}
