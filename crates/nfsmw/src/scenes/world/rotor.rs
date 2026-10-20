//! The police helicopter's rotor for the audio: where the chase's helicopter is, each frame. Spec:
//! `docs/specs/ai-helicopter.md` section 7.

use super::WorldScene;

impl WorldScene {
    /// The helicopter's rotor: its position, nose direction and velocity as the chopper's body has them, in render space.
    /// `None` while no helicopter flies, which stops the rotor.
    pub(super) fn rotor_frame_now(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        let (position, forward, velocity) = self.traffic.as_ref().and_then(|t| t.helicopter_motion())?;
        Some(crate::audio::HeliFrame { dt, position, forward, velocity })
    }
}
