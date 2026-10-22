//! Car-to-car contact: discs that overlap are pushed apart and slowed; a hard touch is reported.

use super::ToyWorld;
use super::bots::distance_sq;
use super::car::CAR_RADIUS;
use crate::SimEvent;

/// A touch at less than this relative speed (m/s) is a nudge and produces no event.
pub const CONTACT_MIN_SPEED: f32 = 1.0;

/// The speed factor each car keeps after a hard touch and after a nudge.
const HARD_KEEP: f32 = 0.7;
const SOFT_KEEP: f32 = 0.95;

impl ToyWorld {
    /// Test every pair in id order, once. Each pair that overlaps is separated half and half along the line between the
    /// centres; the event, when there is one, names the lower id first.
    pub(super) fn resolve_contacts(&mut self, events: &mut Vec<SimEvent>) {
        let ids = self.entity_ids();
        let reach = 2.0 * CAR_RADIUS;
        let bounds = self.config.bounds;
        for (first, &a) in ids.iter().enumerate() {
            for &b in &ids[first + 1..] {
                let (Some(&ca), Some(&cb)) = (self.car(a), self.car(b)) else {
                    continue;
                };
                let d2 = distance_sq(ca.pos, cb.pos);
                if d2 >= reach * reach {
                    continue;
                }
                let dist = d2.sqrt();
                let mut normal = [1.0, 0.0];
                if dist > 1.0e-4 {
                    normal = [(cb.pos[0] - ca.pos[0]) / dist, (cb.pos[1] - ca.pos[1]) / dist];
                }
                let (va, vb) = (ca.velocity(), cb.velocity());
                let relative = distance_sq(va, vb).sqrt();
                let hard = relative > CONTACT_MIN_SPEED;
                if hard {
                    events.push(SimEvent::Contact { a, b, speed: relative });
                }
                let keep = match hard {
                    true => HARD_KEEP,
                    false => SOFT_KEEP,
                };
                let push = (reach - dist) * 0.5;
                if let Some(car) = self.car_mut(a) {
                    car.pos = [car.pos[0] - normal[0] * push, car.pos[1] - normal[1] * push];
                    car.speed *= keep;
                    car.keep_inside(bounds);
                }
                if let Some(car) = self.car_mut(b) {
                    car.pos = [car.pos[0] + normal[0] * push, car.pos[1] + normal[1] * push];
                    car.speed *= keep;
                    car.keep_inside(bounds);
                }
            }
        }
    }
}
