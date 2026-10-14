//! Installed environment context follows the ordered physics positions.
use super::*;

impl WorldScene {
    pub(super) fn audio_state(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.garage_open {
            return None;
        }
        let lens = self.gameplay_lens(1.0);
        let pose = self.drive.as_ref()?.pose();
        let mut state = self.drive.as_mut()?.car_sound()?;
        let (camera_state, view_value) = match self.camera_mode {
            CameraMode::Bumper => (1, 0),
            CameraMode::Hood => (2, 4000),
            _ => (0, 32767),
        };
        state.view = crate::audio::AudioView {
            car_position: pose.position,
            car_forward: pose.rotation * glam::Vec3::X,
            camera_position: lens.position,
            camera_forward: lens.forward,
            camera_state,
            view_value,
            speedbreaker: self.sound_speedbreaker_active,
            speedbreaker_mix: None,
            environment: self
                .audio_environment
                .as_ref()
                .map(|map| map.at(glam::Vec3::from_array(space::to_physics(pose.position))))
                .unwrap_or_default(),
        };
        for tick in &mut state.ticks {
            tick.telemetry.environment = tick
                .telemetry
                .physics_position
                .and_then(|position| self.audio_environment.as_ref().map(|map| map.at(position)));
        }
        state.telemetry.environment = Some(state.view.environment);
        Some(state)
    }
}
