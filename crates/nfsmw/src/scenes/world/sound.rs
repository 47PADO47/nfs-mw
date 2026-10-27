//! Installed environment context follows the ordered physics positions.
use super::race::Stage;
use super::*;
use blackbox_race::Phase;

/// Whether a story cutscene owns the sound: a movie plays, or the NIS of a race's intro does
/// (`docs/specs/prologue-car-and-audio.md`). The movie's and the NIS's own streams are the sound then, not the car idling
/// on the grid under a camera that is far from it.
fn cutscene_owns_sound(movie: bool, nis: bool, race_intro: bool) -> bool {
    movie || (nis && race_intro)
}

fn pre_race_audio(rolling: bool, stage: &Stage, phase: Option<Phase>) -> bool {
    if rolling {
        return false;
    }
    match stage {
        Stage::Loading { .. } | Stage::Intro { .. } | Stage::Staged { .. } => true,
        Stage::Running => matches!(phase, Some(Phase::Staging | Phase::Countdown { .. })),
        _ => false,
    }
}

fn set_pre_race(state: &mut crate::audio::CarSoundState, before_go: bool) {
    state.input.pre_race = before_go;
    if before_go {
        for tick in &mut state.ticks {
            tick.input.pre_race = true;
        }
    }
    // After GO preserve each completed tick's staging flag, including a catch-up frame spanning GO.
}

impl WorldScene {
    /// Only local engine voices return display feedback; absent voices clear the corresponding seat.
    pub(super) fn accept_car_visual_rpm(&mut self, player: usize, fraction: Option<f32>) {
        match player {
            0 => {
                if let Some(drive) = self.players.lead_mut() {
                    drive.note_visual_rpm(fraction);
                }
            }
            1 => {
                if let Some(guest) = self.players.guests_mut().next() {
                    guest.drive.note_visual_rpm(fraction);
                }
            }
            _ => {}
        }
    }

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
        let before_go = self.race.as_ref().is_some_and(|race| {
            pre_race_audio(race.origin.race.params.rolling_start, &race.stage, race.engine.as_ref().map(|e| e.phase()))
        });
        set_pre_race(&mut state, before_go);
        let lens = self.gameplay_lens(1.0);
        let (camera_state, view_value) = match self.camera_mode {
            CameraMode::Bumper => (1, 0),
            CameraMode::Hood => (2, 4000),
            _ => (0, 32767),
        };
        state.view = crate::audio::AudioView {
            rain_intensity: self.rain.intensity(),
            weather_streaming: !self.residency.complete(),
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
        state.nitrous_fx = self.nitrous_fx.clone();
        state
    }
}

#[cfg(test)]
mod tests {
    use super::{Phase, Stage, cutscene_owns_sound, pre_race_audio, set_pre_race};

    #[test]
    fn pre_race_visual_audio_tracks_standing_countdown_and_excludes_rolling_starts() {
        assert!(pre_race_audio(false, &Stage::Loading { waited: 0.0 }, None));
        assert!(pre_race_audio(false, &Stage::Staged { lead: 180 }, Some(Phase::Staging)));
        assert!(pre_race_audio(false, &Stage::Running, Some(Phase::Countdown { left: 1.0 })));
        assert!(!pre_race_audio(false, &Stage::Running, Some(Phase::Racing)));
        assert!(!pre_race_audio(true, &Stage::Running, Some(Phase::Countdown { left: 1.0 })));
        assert!(!pre_race_audio(true, &Stage::Loading { waited: 0.0 }, None));
    }

    #[test]
    fn a_frame_spanning_go_retains_each_ticks_pre_race_phase() {
        use crate::audio::{CarSoundState, CarSoundTick};
        let mut state = CarSoundState {
            nitrous_fx: None,
            engine: Default::default(),
            car: "fixture".into(),
            upgrades: Default::default(),
            input: Default::default(),
            telemetry: Default::default(),
            events: Vec::new(),
            scrape: None,
            surface: 0,
            view: Default::default(),
            ticks: [true, false]
                .map(|pre_race| CarSoundTick {
                    sim_dt: 1.0 / 60.0,
                    telemetry: Default::default(),
                    input: blackbox_carsound::CarInput { pre_race, ..Default::default() },
                    contacts: Vec::new(),
                    surface: 0,
                })
                .into(),
        };
        set_pre_race(&mut state, false);
        assert!(!state.input.pre_race);
        assert_eq!(state.ticks.iter().map(|tick| tick.input.pre_race).collect::<Vec<_>>(), [true, false]);
        set_pre_race(&mut state, true);
        assert!(state.input.pre_race && state.ticks.iter().all(|tick| tick.input.pre_race));
    }

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
