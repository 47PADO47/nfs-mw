//! The spectator's camera (MP-41): a spectator has no car of its own, so the world shows the free camera, held behind
//! the racer the session follows (`ClientSession::follow_target`, changed with `follow_next` and `follow_previous`).

use glam::Vec3;
use nfsmw_sim::EntityId;

use super::super::{View, WorldScene};
use super::frame::car_pose;

/// How far behind and above the followed car the camera sits, metres.
const BEHIND: f32 = 9.0;
const ABOVE: f32 = 3.5;
/// How fast the camera catches up with its place, per second.
const CATCH_UP: f32 = 4.0;
/// The camera looks slightly down, radians.
const PITCH: f32 = -0.2;

impl WorldScene {
    /// Puts the free camera behind `target` (an entity of this frame's display list), easing towards it.
    pub(super) fn follow_spectated(&mut self, target: Option<EntityId>, dt: f32) {
        self.view = View::Fly;
        let Some(mode) = self.net_client.as_ref() else { return };
        let followed = target.and_then(|target| mode.display.iter().find(|(id, _)| *id == target));
        let Some((_, drawn)) = followed else { return };
        let pose = car_pose(drawn.view.position, drawn.view.orientation);
        let forward = pose.rotation * Vec3::X;
        let wanted = pose.position - forward * BEHIND + Vec3::Z * ABOVE;
        let blend = (dt * CATCH_UP).clamp(0.0, 1.0);
        self.camera.position = self.camera.position.lerp(wanted, blend);
        self.camera.yaw = forward.y.atan2(forward.x);
        self.camera.pitch = PITCH;
    }
}
