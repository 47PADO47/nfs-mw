//! Ordered ground support of the body box, used when suspension bottoms out or a car overturns.

use super::Vehicle;
use crate::ground::Ground;
use crate::rigid_body::BOX_CORNERS;

impl Vehicle {
    pub(super) fn collide_body_with_ground(&mut self, ground: &dyn Ground) -> usize {
        let half = self.body.dimension() + self.body.spec().collision_box_pad;
        let points = BOX_CORNERS.map(|corner| self.body.position + self.body.rotation() * (half * corner));
        let mut contacts: Vec<_> =
            points.into_iter().filter_map(|point| self.body.ground_contact(point, ground, crate::FIXED_STEP)).collect();
        self.body.resolve_ground(&mut contacts);
        contacts.len()
    }
}
