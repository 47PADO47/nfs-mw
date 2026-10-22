//! Installed environment context follows the ordered physics positions.
use super::race::Stage;
use super::*;

/// Whether a story cutscene owns the sound: a movie plays, or the NIS of a race's intro does
/// (`docs/specs/prologue-car-and-audio.md`). The movie's and the NIS's own streams are the sound then, not the car idling
/// on the grid under a camera that is far from it.
fn cutscene_owns_sound(movie: bool, nis: bool, race_intro: bool) -> bool {
    movie || (nis && race_intro)
}

impl WorldScene {
    /// The lead's car, for its engine voice.
    pub(super) fn audio_state(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.garage_open {
            return None;
        }
        if self.car_silenced() {
            return None;
        }
        let pose = self.players.lead()?.pose();
        let state = self.players.lead_mut()?.car_sound()?;
        Some(self.listened_car_state(state, pose))
    }

    /// The first guest's car, for player 2's own engine voice in a co-op session. The listener is still player 1's
    /// camera: one listener hears both voices.
    pub(super) fn guest_audio_state(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.garage_open {
            return None;
        }
        if self.car_silenced() {
            return None;
        }
        let guest = self.players.guests_mut().next()?;
        let pose = guest.drive.pose();
        let state = guest.drive.car_sound()?;
        Some(self.listened_car_state(state, pose))
    }

    /// A story cutscene owns the sound: no engine voice plays under it.
    fn car_silenced(&self) -> bool {
        let race_intro = self.race.as_ref().is_some_and(|race| matches!(race.stage, Stage::Intro { .. }));
        cutscene_owns_sound(self.story_movie.is_playing(), self.nis.is_playing(), race_intro)
    }

    /// The car's sound with the view it is heard from: the car at `pose`, the gameplay lens of the frame and the
    /// environment at the car.
    fn listened_car_state(
        &self,
        mut state: crate::audio::CarSoundState,
        pose: crate::scenes::world::drive::CarPose,
    ) -> crate::audio::CarSoundState {
        let lens = self.gameplay_lens(1.0);
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
        state
    }
}

#[cfg(test)]
mod tests {
    use super::cutscene_owns_sound;

    #[test]
    fn a_movie_or_the_intro_nis_of_a_race_silences_the_car() {
        assert!(cutscene_owns_sound(true, false, false));
        assert!(cutscene_owns_sound(true, false, true));
        assert!(cutscene_owns_sound(false, true, true));
    }

    #[test]
    fn other_scenes_keep_the_car_audible() {
        assert!(!cutscene_owns_sound(false, false, false));
        assert!(!cutscene_owns_sound(false, false, true));
        // The bust's arrest scene is a NIS outside any race intro.
        assert!(!cutscene_owns_sound(false, true, false));
    }
}
