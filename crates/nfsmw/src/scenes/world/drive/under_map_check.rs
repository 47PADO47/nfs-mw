//! The host's side of the under-the-map watch (`under_map.rs`, `docs/specs/ride6-feel-round14.md` section 4): what is looked at each physics
//! step, and the way back to the road.

use blackbox_collision::CollisionWorld;
use glam::Vec3;

use super::Drive;
use super::rider::Rider;
use super::under_map::{BELOW_M, COVER_M, Probe};
use crate::scenes::world::space;

/// How far above a body the ray that looks for the ground below it starts (m): a body that has sunk less than this into a surface has
/// that surface for its ground.
const BELOW_START_M: f32 = 0.8;

/// What is around `at` (render space) for a body of speed `speed` (m/s) and the last floor `floor`.
fn probe(
    collision: &CollisionWorld,
    at: Vec3,
    (speed, floor): (f32, Option<f32>),
    (rider, finite): (bool, bool),
) -> Probe {
    let finite = finite && at.is_finite();
    let (ground_below, covered) = match finite {
        true => (
            space::ground_below(collision, at.x, at.y, at.z + BELOW_START_M, at.z - BELOW_M).is_some(),
            space::ground_below(collision, at.x, at.y, at.z + COVER_M, at.z).is_some(),
        ),
        false => (false, false),
    };
    Probe { height: at.z, speed, floor, rider, ground_below, covered, finite }
}

impl Drive {
    /// The height of the last place that held the bike up (render space): its trail's newest place, or the last road it stood on.
    fn support_floor(&self) -> Option<f32> {
        self.reset.trail.newest().map(|crumb| crumb.position.z).or(self.last_good.map(|good| good.position.z))
    }

    /// Put a bike (or its rider, who has no road of his own) back on the road at once: the reset of the trail, as the reset button does it,
    /// else the last road it stood on, else the road nearest `near`.
    pub(super) fn put_back_on_road(&mut self, near: Vec3) {
        if self.request_reset() || self.restore_last_good() {
            return;
        }
        self.respawn_near([near.x, near.y], None);
    }

    /// One physics step of the watch over a bike and its thrown rider: a bike that is under the map, or whose rider is (he has no wheels
    /// and nobody else would notice), or whose state is not usable, is put back on the road within a third of a second.
    pub(super) fn check_under_map(&mut self, collision: &CollisionWorld, finite: bool, dt: f32) {
        let Some(sim) = self.sim.as_ref().filter(|sim| sim.is_bike()) else { return };
        if self.request.is_some() {
            return;
        }
        let floor = self.support_floor();
        let bike = self.current.position;
        let bike_probe = probe(collision, bike, (sim.effect_velocity().length(), floor), (false, finite));
        if self.under_map[0].update(&bike_probe, dt) {
            log::warn!(
                "the bike went under the map at ({:.0}, {:.0}, {:.0}); putting it back on the road",
                bike.x,
                bike.y,
                bike.z
            );
            self.under_map = Default::default();
            self.put_back_on_road(bike);
            return;
        }
        let Some((_, rider)) = self.rider.off_bike() else {
            self.under_map[1].clear();
            return;
        };
        let rider_probe = probe(collision, rider, (self.rider_cam.velocity().length(), floor), (true, true));
        if self.under_map[1].update(&rider_probe, dt) {
            log::warn!(
                "the rider went under the map at ({:.0}, {:.0}, {:.0}); putting the bike and him back on the road",
                rider.x,
                rider.y,
                rider.z
            );
            self.under_map = Default::default();
            self.put_back_on_road(bike);
        }
    }
}

/// The height of the ground under the thrown rider, for the camera that follows him (`RiderCam::set_floor`): none while he rides.
pub(super) fn rider_floor(rider: &Rider, collision: &CollisionWorld) -> Option<f32> {
    let (_, at) = rider.off_bike()?;
    space::ground_below(collision, at.x, at.y, at.z + 1.0, at.z - 2.0)
}
