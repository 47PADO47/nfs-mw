//! Ordered ground support of the body box, used when suspension bottoms out or a car overturns.

use super::Vehicle;
use crate::ground::Ground;
use crate::rigid_body::BOX_CORNERS;

impl Vehicle {
    pub(super) fn collide_body_with_ground(&mut self, ground: &dyn Ground, dt: f32) -> usize {
        let half = self.body.dimension() + self.body.spec().collision_box_pad;
        let points = BOX_CORNERS.map(|corner| self.body.position + self.body.rotation() * (half * corner));
        let mut contacts: Vec<_> =
            points.into_iter().filter_map(|point| self.body.ground_contact(point, ground, dt)).collect();
        self.body.resolve_ground(&mut contacts);
        let count = contacts.len();
        self.body_contacts = contacts;
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputState, VehicleSpec};
    use glam::Vec3;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Queries(RefCell<Vec<(Vec3, f32)>>);

    impl Ground for Queries {
        fn hit(&self, origin: Vec3, _: Vec3, distance: f32) -> Option<crate::ground::GroundHit> {
            self.0.borrow_mut().push((origin, distance));
            None
        }
    }

    #[test]
    fn body_probes_use_the_vehicle_step_delta_at_normal_and_slowed_rates() {
        for dt in [crate::FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
            let mut car = Vehicle::new(VehicleSpec::example());
            car.body.position = Vec3::Y * 5.0;
            car.body.linear_velocity = Vec3::new(20.0, -8.0, 0.0);
            let actual = Queries::default();
            car.step(dt, &InputState::default(), &actual);

            // With no contacts, the final body pose is also the body-probe pose. Compare the adapter
            // to the existing body query contract, independently of the wheel-probe queries before it.
            let expected = Queries::default();
            let half = car.body.dimension() + car.body.spec().collision_box_pad;
            for corner in BOX_CORNERS {
                let point = car.body.position + car.body.rotation() * (half * corner);
                car.body.ground_contact(point, &expected, dt);
            }
            let actual = actual.0.borrow();
            assert_eq!(&actual[actual.len() - 8..], expected.0.borrow().as_slice(), "dt {dt}");
        }
    }
}
