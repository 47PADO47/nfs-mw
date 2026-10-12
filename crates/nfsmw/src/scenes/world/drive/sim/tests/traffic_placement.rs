//! Traffic must begin above its support, without a normal impact velocity or a stale sweep pose.

use super::*;
use blackbox_vehicle::ground::{GroundHit, SurfaceGrip};
use blackbox_vehicle::rigid_body::BOX_CORNERS;

struct Plane {
    point: Vec3,
    normal: Vec3,
}

impl Ground for Plane {
    fn hit(&self, origin: Vec3, direction: Vec3, limit: f32) -> Option<GroundHit> {
        let denominator = self.normal.dot(direction);
        if denominator >= -f32::EPSILON {
            return None;
        }
        let distance = self.normal.dot(self.point - origin) / denominator;
        (distance >= 0.0 && distance <= limit).then_some(GroundHit {
            distance,
            normal: self.normal,
            surface: SurfaceGrip::DEFAULT,
        })
    }
}

#[test]
fn traffic_starts_clear_of_banked_and_sloping_ground_without_an_initial_impact() {
    let point = Vec3::from_array(space::to_physics(spawn_at(0.0).position));
    for normal in [Vec3::Y, Vec3::new(0.15, 1.0, -0.2).normalize(), Vec3::new(-0.3, 1.0, 0.1).normalize()] {
        let ground = Plane { point, normal };
        for heading in [0.0, std::f32::consts::FRAC_PI_2, -1.2] {
            let mut car = sim();
            assert!(car.place_traffic_moving(&ground, spawn_at(heading), 12.0));
            let body = car.body();
            let forward = space::to_render(body.rotation().z_axis.to_array());
            let heading_error = forward.truncate().normalize().distance(glam::Vec2::new(heading.cos(), heading.sin()));
            assert!(heading_error < 1e-5, "horizontal heading changed by {heading_error}");
            assert!(body.linear_velocity.dot(normal).abs() < 1e-4, "spawn velocity strikes its support");
            assert!((body.linear_velocity.length() - 12.0).abs() < 1e-4);
            let padded = body.dimension() + body.spec().collision_box_pad;
            for corner in BOX_CORNERS {
                let point_on_body = body.position + body.rotation() * (padded * corner);
                assert!(normal.dot(point_on_body - point) >= -1e-5, "body starts inside its support");
            }
            assert_eq!(car.previous_contact_pose, (body.position, body.orientation), "first sweep has stale placement");
            let before = body.position;
            car.step(&DriveInput::default(), &ground, None);
            let normal_shift = normal.dot(car.body().position - before);
            assert!(normal_shift.abs() < 1e-3, "first tick depenetrates or lifts the body: {normal_shift}");
            assert!(car.is_finite());
        }
    }
}

#[test]
fn traffic_flat_placement_keeps_the_existing_unloaded_clearance_and_speed() {
    for speed in [0.0, 15.0] {
        let (mut old, mut traffic) = (sim(), sim());
        let ground = FlatGround::new(5.0);
        assert!(old.place_moving(&ground, spawn_at(0.8), speed));
        assert!(traffic.place_traffic_moving(&ground, spawn_at(0.8), speed));
        assert!(old.body().position.distance(traffic.body().position) < 1e-5);
        assert!(old.body().linear_velocity.distance(traffic.body().linear_velocity) < 1e-5);
        assert_eq!(old.telemetry().gear, traffic.telemetry().gear);
        assert_eq!(old.telemetry().rpm, traffic.telemetry().rpm);
    }
}

#[test]
fn missing_traffic_ground_does_not_change_an_existing_placement() {
    let mut car = sim();
    assert!(car.place_traffic_moving(&FlatGround::new(5.0), spawn_at(0.3), 10.0));
    let before = (car.body().position, car.body().orientation, car.body().linear_velocity);
    assert!(!car.place_traffic_moving(&NoGround, spawn_at(-0.5), 20.0));
    assert_eq!(before, (car.body().position, car.body().orientation, car.body().linear_velocity));
}
