//! The plane a roadblock prop lies on, fitted from the surface at its ends.

use blackbox_vehicle::Ground;
use blackbox_vehicle::ground::{GroundHit, SurfaceGrip};
use glam::Vec3;

use super::fitted_surface;

/// A plane through `point` with `normal`, whose hits report `reported` as their normal (a seam's odd face).
struct Plane {
    point: Vec3,
    normal: Vec3,
    reported: Vec3,
}

impl Ground for Plane {
    fn hit(&self, origin: Vec3, direction: Vec3, limit: f32) -> Option<GroundHit> {
        let denominator = self.normal.dot(direction);
        if denominator.abs() <= f32::EPSILON {
            return None;
        }
        let distance = self.normal.dot(self.point - origin) / denominator;
        (distance >= 0.0 && distance <= limit).then_some(GroundHit {
            distance,
            normal: self.reported,
            surface: SurfaceGrip::DEFAULT,
        })
    }
}

#[test]
fn a_level_road_gives_a_level_strip_whatever_the_face_under_its_centre_says() {
    let tilted = Vec3::new(0.14, 1.0, 0.0).normalize();
    let road = Plane { point: Vec3::ZERO, normal: Vec3::Y, reported: tilted };
    let (height, normal) = fitted_surface(&road, Vec3::new(5.0, 0.0, 5.0), 0.3, [0.4, 3.2]).unwrap();
    assert!(height.abs() < 1e-4, "{height}");
    assert!((normal - Vec3::Y).length() < 1e-4, "{normal:?}");
}

#[test]
fn a_banked_road_tilts_the_strip_with_it() {
    let bank = Vec3::new(0.0, 1.0, 0.1).normalize();
    let road = Plane { point: Vec3::ZERO, normal: bank, reported: bank };
    let (height, normal) = fitted_surface(&road, Vec3::ZERO, 0.0, [0.4, 3.2]).unwrap();
    assert!(height.abs() < 1e-3, "{height}");
    assert!((normal - bank).length() < 1e-3, "{normal:?}");
}

#[test]
fn no_road_under_an_end_keeps_the_single_point_answer() {
    struct Nothing;
    impl Ground for Nothing {
        fn hit(&self, _: Vec3, _: Vec3, _: f32) -> Option<GroundHit> {
            None
        }
    }
    assert!(fitted_surface(&Nothing, Vec3::ZERO, 0.0, [0.4, 3.2]).is_none());
}

#[test]
fn a_kerb_under_one_end_leaves_the_strip_on_the_road() {
    /// A level road whose surface steps up 0.15 m past z 2.9: a kerb under the strip's far end.
    struct Kerb;
    impl Ground for Kerb {
        fn hit(&self, origin: Vec3, direction: Vec3, limit: f32) -> Option<GroundHit> {
            let top = if origin.z > 2.9 { 0.15 } else { 0.0 };
            let distance = (origin.y - top) / -direction.y;
            (direction.y < 0.0 && distance >= 0.0 && distance <= limit).then_some(GroundHit {
                distance,
                normal: Vec3::Y,
                surface: SurfaceGrip::DEFAULT,
            })
        }
    }
    let (height, normal) = fitted_surface(&Kerb, Vec3::ZERO, 0.0, [0.4, 3.2]).unwrap();
    assert!(height.abs() < 1e-3, "{height}");
    assert!((normal - Vec3::Y).length() < 1e-3, "the kerb does not tilt the strip: {normal:?}");
}
