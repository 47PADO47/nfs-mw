//! The police helicopter's rotor for the audio: where the chase's helicopter is, each frame. Spec:
//! `docs/specs/ai-helicopter.md` section 7.
use glam::Vec3;

use super::WorldScene;

impl WorldScene {
    /// The helicopter's rotor: its position from the traffic's chopper, render space; its velocity is the change since
    /// the last frame. The heading is flat, from that velocity: the chopper's orientation has no accessor yet.
    pub(super) fn rotor_frame_now(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        let position = self.traffic.as_ref().and_then(|t| t.camera_chopper_positions().first().copied());
        let previous = std::mem::replace(&mut self.rotor_previous, position);
        let position = position?;
        let velocity = match previous {
            Some(previous) if dt > 0.0 => (position - previous) / dt,
            _ => Vec3::ZERO,
        };
        let forward = Vec3::new(velocity.x, velocity.y, 0.0).normalize_or_zero();
        Some(crate::audio::HeliFrame { dt, position, forward, velocity })
    }
}
