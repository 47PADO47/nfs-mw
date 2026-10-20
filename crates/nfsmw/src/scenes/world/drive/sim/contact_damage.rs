//! Ground collision callbacks use their normal accumulator without the object multiplier.

use super::CarSim;
use crate::scenes::world::events::{DamageHit, OtherBody};

impl CarSim {
    pub(super) fn append_ground_damage(&self, out: &mut Vec<DamageHit>) {
        let body = self.vehicle.body();
        out.extend(self.vehicle.body_contacts().iter().filter(|c| c.impulse_delta_v > 0.0).map(|c| {
            let mut hit = DamageHit::capture(body, c.point, c.normal, c.closing_speed, OtherBody::default());
            hit.arm = c.local_arm;
            hit.force = c.impulse_delta_v * body.mass();
            hit.before_velocity = c.before_velocity;
            hit
        }));
    }
}
