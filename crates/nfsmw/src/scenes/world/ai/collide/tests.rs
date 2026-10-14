use blackbox_vehicle::VehicleSpec;
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};

use super::*;

#[test]
fn vertical_jump_velocity_does_not_trigger_a_traffic_driveby() {
    assert_eq!(planar_relative_speed(Vec3::Y * 50.0, Vec3::ZERO), 0.0);
    assert_eq!(planar_relative_speed(Vec3::new(3.0, 50.0, 4.0), Vec3::ZERO), 5.0);
}

fn sim(at: Vec3) -> CarSim {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    let mut sim = CarSim::new(
        CarPhysics {
            tire_integration: Default::default(),
            jump_config: Default::default(),
            spec,
            bounds,
            walls: WallSpec::default(),
            geometry: None,
            events: None,
            reactions: None,
        },
        [0.2; 4],
    );
    sim.configure_ai();
    sim.body_mut().place(at, Quat::IDENTITY);
    sim
}

#[test]
fn a_nonfinite_partner_cannot_separate_a_valid_car_before_impulse_validation() {
    for invalid_first in [false, true] {
        let mut healthy = sim(Vec3::ZERO);
        let mut invalid = sim(Vec3::X);
        invalid.body_mut().angular_velocity = Vec3::splat(f32::NAN);
        assert!(healthy.is_finite() && !invalid.is_finite());
        let before = healthy.body().position;
        let (a, b) = match invalid_first {
            true => (&mut invalid, &mut healthy),
            false => (&mut healthy, &mut invalid),
        };
        if near(a, b) {
            a.collide_with(b);
        }
        assert_eq!(healthy.body().position, before, "invalid_first: {invalid_first}");
        assert!(healthy.is_finite());
    }
}

#[test]
fn valid_overlapping_cars_still_get_contact_separation() {
    let mut a = sim(Vec3::ZERO);
    let mut b = sim(Vec3::X);
    assert!(near(&a, &b));
    a.collide_with(&mut b);
    assert!(a.body().position.x < 0.0 && b.body().position.x > 1.0);
}
