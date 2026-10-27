use super::*;
use crate::ground::{GroundHit, SurfaceGrip, WheelPlane};

struct ContactPlane(Vec3);

impl Ground for ContactPlane {
    fn wheel_plane(&self, _: usize, _: Vec3, _: f32, _: bool) -> Option<WheelPlane> {
        Some(WheelPlane { normal: self.0, separation: 0.25, surface: SurfaceGrip::DEFAULT })
    }

    fn hit(&self, _: Vec3, _: Vec3, _: f32) -> Option<GroundHit> {
        None
    }
}

#[test]
fn bottom_out_recovery_moves_body_and_all_patches_without_requiring_tire_load() {
    for normal in [Vec3::Y, Vec3::new(0.99_f32.sqrt(), 0.1, 0.0)] {
        for dt in [1.0 / 60.0, 1.0 / 144.0, 1.0 / 240.0, 1.0 / 576.0] {
            let mut car = Vehicle::new(VehicleSpec::example());
            let old_y = car.position().y;
            let mut lift = 0.0_f32;
            let mut patches = [Vec3::ZERO; 4];
            for (i, patch) in patches.iter_mut().enumerate() {
                let a = car.spec.chassis.axle(i / 2, 0.0);
                let c = crate::suspension::compression(a.ride, a.travel, normal.y, 0.25);
                lift = lift.max(c.lift_need);
                *patch = car.geometry.arms[i];
                patch.y += c.value - a.ride;
            }
            assert!(lift > 0.0);
            car.step(dt, &InputState::default(), &ContactPlane(normal));
            assert_eq!(car.position().y, old_y + lift);
            assert_eq!(car.wheels_on_ground(), if normal.y > 0.2 { 4 } else { 0 });
            for (i, patch) in patches.iter().enumerate() {
                assert_eq!(car.wheel(i).position, *patch + Vec3::Y * lift);
                assert_eq!(car.wheel(i).on_ground, normal.y > 0.2);
            }
        }
    }
}
