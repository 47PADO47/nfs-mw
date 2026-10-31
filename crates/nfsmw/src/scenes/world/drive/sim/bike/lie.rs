//! A bike that lost its rider lies on its side until the rider has got up, and is then stood up (`docs/specs/ride6-rider-getup.md`).
//!
//! The bike does not stand up on its own clock: once it has lain still it *waits* for its rider, and the host (`drive/rider/fall.rs`)
//! tells it every step whether he is ready (his get-up has played to its end). Until then the bike stays fallen, which keeps the
//! rider off it and the controls from it (a fallen bike ignores the throttle and the stick). Placing the bike again by hand (the
//! reset button) does not wait for anybody.

use blackbox_vehicle::Ground;
use glam::Vec3;

use super::BikeRide;

/// A bike that lost its rider lies this long before it may be stood up again (s). A modelling choice (`ride6-rider-fall.md`): the
/// reference lets the rider get up, walk to the bike and raise it, which takes longer and is not modelled.
pub(super) const GET_UP_AFTER: f32 = 2.0;
/// ... and only once it has come to a stop slower than this (m/s). The bike slides on its collision box until the ground's friction
/// stops it: nothing else slows it.
pub(super) const GET_UP_SPEED: f32 = 1.5;

impl BikeRide {
    /// A bike that lost its rider: it slides on its side (the double meets the ground, `finish_body`) and, once it has lain still
    /// for a while *and its rider is ready*, stands up where it is with the rider back on it.
    pub(super) fn lie(&mut self, dt: f32, ground: &dyn Ground) {
        self.down += dt;
        self.waiting = self.down >= GET_UP_AFTER && self.bike.velocity().length() <= GET_UP_SPEED;
        if !self.waiting || !self.rider_ready {
            return;
        }
        let position = self.bike.position();
        let forward = self.bike.orientation() * Vec3::Z;
        let yaw = forward.x.atan2(forward.z);
        if self.bike.place_on_ground(ground, position.x, position.z, position.y + 2.0, yaw) {
            self.down = 0.0;
            self.waiting = false;
        }
    }

    /// Whether the bike has lain still long enough to be stood up and is only waiting for its rider.
    pub(in crate::scenes::world) fn waiting_for_rider(&self) -> bool {
        self.waiting
    }

    /// The rider says whether he is ready for the bike to be stood up (he has got up). Said again every step.
    pub(in crate::scenes::world) fn set_rider_ready(&mut self, ready: bool) {
        self.rider_ready = ready;
    }
}
