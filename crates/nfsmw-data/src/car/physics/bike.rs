//! The two-wheel part of a bike's physics (docs/specs/vehicle-two-wheel-integration.md).
//!
//! A bike is a [`CarPhysics`] like any other car: its four-wheel `spec` is the body that stands in for it in the game's
//! wall and object contacts (built from the stats derived from the bike's data, with the collision box of its own
//! `BOUNDS.BIN`), and [`BikePhysics`] is what moves it. The game steps the two-wheel dynamics and carries the result into
//! that body.

use std::sync::Arc;

use blackbox_vehicle::bike::BikeSpec;

use super::{CarBounds, CarPhysics};
use crate::car::mods;

/// The data of a bike's two-wheel dynamics, with where its body sits on the model.
#[derive(Debug, Clone)]
pub struct BikePhysics {
    /// What the bike is: chassis, wheels, suspension, engine, gearbox, brakes and steering.
    pub spec: Arc<BikeSpec>,
    /// Where the bike's body origin (on the ground, midway between the contact patches at rest) is, forward of the model's
    /// origin (m).
    pub origin_x: f32,
}

impl CarPhysics {
    /// The bike `spec` with the collision box `bounds`. `origin_x` is where its body origin is on the model (see
    /// [`BikePhysics::origin_x`]).
    ///
    /// The body for the game's contacts is the car physics of the stats derived from the bike
    /// ([`mods::bike::derived_stats`]); like any mod car's it has no contact geometry, events, reactions or effect source
    /// of its own, which the caller fills from the car's bounds.
    pub fn from_bike(spec: Arc<BikeSpec>, origin_x: f32, bounds: CarBounds) -> Self {
        let stats = mods::bike::derived_stats(&spec);
        let mut physics = Self::from_stats(&stats, bounds);
        physics.bike = Some(BikePhysics { spec, origin_x });
        physics
    }

    /// Whether the car is a bike.
    pub fn is_bike(&self) -> bool {
        self.bike.is_some()
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    const BOUNDS: CarBounds =
        CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.0) };

    #[test]
    fn a_bike_is_a_car_physics_with_its_two_wheel_data() {
        let physics = CarPhysics::from_bike(Arc::new(BikeSpec::example()), 0.1, BOUNDS);
        assert!(physics.is_bike());
        let bike = physics.bike.as_ref().unwrap();
        assert_eq!(bike.origin_x, 0.1);
        assert_eq!(bike.spec.chassis.wheelbase, BikeSpec::example().chassis.wheelbase);
        // The body in the game's contacts is the bike's box and mass.
        assert_eq!(physics.spec.dimension, BOUNDS.half_dimensions);
        assert_eq!(physics.spec.mass, BikeSpec::example().total_mass());
        assert!(physics.geometry.is_none() && physics.events.is_none());
    }

    #[test]
    fn a_car_is_not_a_bike() {
        let physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
        assert!(!physics.is_bike());
    }
}
