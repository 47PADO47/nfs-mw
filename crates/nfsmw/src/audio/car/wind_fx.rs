//! The wind of a mod's wind variant (`docs/modding/effect-variants.md`, "Profile format of the wind"): looping recordings that
//! play all the time the car is driven, each as loud as a curve of the car's speed says.
//!
//! The files are read from the install's layout (the mod's `files/` overlay), once per profile. While the profile's loops
//! are playing, [`WindVoice::update`] answers true and the car's ambient sounds keep the game's own wind (the `FX_WIND` module of
//! the 2005 game) silent. A profile whose files cannot be read leaves the game's wind playing: a variant never makes the wind
//! mute.

use std::sync::Arc;
use std::time::Duration;

use blackbox_modding::content::WindProfile;
use kira::sound::static_sound::StaticSoundHandle;
use kira::{PlaybackRate, Tween};

use super::own_engine::parse_wav;
use crate::audio::volume::decibels;
use crate::audio::{Audio, Group, pcm};

/// A change of level is spread over this long, so a frame's step is not heard as a click.
const SMOOTH: Duration = Duration::from_millis(50);
/// Miles an hour in a metre a second.
const MPH_PER_MPS: f32 = 2.236_936;

/// The loops of the wind variant of one car.
#[derive(Default)]
pub struct WindVoice {
    /// The profile `loops` were started for.
    loaded_for: Option<Arc<WindProfile>>,
    /// One handle for each layer of the profile, in order; empty when none is playing.
    loops: Vec<StaticSoundHandle>,
}

impl WindVoice {
    /// One step: follow the profile (`fx`, the mod variant's, or `None`) and set each loop's level for the car's `speed`
    /// (metres a second). Returns whether the profile's loops stand in for the game's own wind.
    pub fn update(&mut self, audio: &mut Audio, fx: Option<&Arc<WindProfile>>, speed: f32) -> bool {
        if !self.same_profile(fx) {
            self.stop(Duration::from_millis(120));
            self.loops = fx.map(|profile| start(audio, profile)).unwrap_or_default();
            self.loaded_for = fx.cloned();
        }
        let Some(profile) = self.loaded_for.as_ref().filter(|_| !self.loops.is_empty()) else { return false };
        let mph = mph(speed);
        let tween = Tween { duration: SMOOTH, ..Tween::default() };
        for (layer, handle) in profile.layers.iter().zip(self.loops.iter_mut()) {
            handle.set_volume(decibels(layer.level_at(mph) * profile.volume), tween);
        }
        true
    }

    fn same_profile(&self, wanted: Option<&Arc<WindProfile>>) -> bool {
        match (&self.loaded_for, wanted) {
            (None, None) => true,
            (Some(have), Some(want)) => Arc::ptr_eq(have, want) || **have == **want,
            _ => false,
        }
    }

    fn stop(&mut self, fade: Duration) {
        for mut handle in self.loops.drain(..) {
            handle.stop(Tween { duration: fade, ..Tween::default() });
        }
    }
}

impl Drop for WindVoice {
    /// A kira handle that is dropped leaves its sound playing, so the loops are stopped here.
    fn drop(&mut self) {
        self.stop(Duration::from_millis(120));
    }
}

/// The car's speed in miles an hour, whichever way it goes; 0 for a number that is not finite.
fn mph(speed: f32) -> f32 {
    match speed.is_finite() {
        true => speed.abs() * MPH_PER_MPS,
        false => 0.0,
    }
}

/// Read and start every loop of the profile, silent. A file that cannot be read or played stops the ones started and gives
/// none, so the game's own wind stays.
fn start(audio: &mut Audio, profile: &WindProfile) -> Vec<StaticSoundHandle> {
    let mut handles: Vec<StaticSoundHandle> = Vec::with_capacity(profile.layers.len());
    for layer in &profile.layers {
        match start_layer(audio, &layer.file, layer.rate) {
            Ok(handle) => handles.push(handle),
            Err(error) => {
                log::warn!("the wind variant's loops are not used, the game's own wind plays: {error}");
                for mut started in handles {
                    started.stop(Tween::default());
                }
                return Vec::new();
            }
        }
    }
    handles
}

fn start_layer(audio: &mut Audio, file: &str, rate: f32) -> Result<StaticSoundHandle, String> {
    let bytes = audio.dir.read(file).map_err(|error| format!("{file}: {error}"))?;
    let wav = parse_wav(&bytes).map_err(|error| format!("{file}: {error}"))?;
    let mut handle =
        audio.play(Group::Sfx, pcm::sound(&wav).volume(decibels(0.0))).map_err(|error| format!("{file}: {error}"))?;
    handle.set_playback_rate(PlaybackRate(f64::from(rate)), Tween::default());
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use blackbox_modding::content::WindLayer;

    use super::*;

    fn profile(volume: f32) -> Arc<WindProfile> {
        let layer =
            WindLayer { file: "WIND/A/W.WAV".into(), volume: 1.0, rate: 1.0, gain: vec![(0.0, 0.0), (50.0, 1.0)] };
        Arc::new(WindProfile { volume, layers: vec![layer] })
    }

    #[test]
    fn the_speed_is_in_miles_an_hour_and_never_negative() {
        assert!((mph(10.0) - 22.36936).abs() < 1e-3);
        assert_eq!(mph(-10.0), mph(10.0));
        assert_eq!((mph(f32::NAN), mph(f32::INFINITY)), (0.0, 0.0));
    }

    #[test]
    fn a_voice_with_no_profile_is_the_same_as_none() {
        let voice = WindVoice::default();
        assert!(voice.same_profile(None));
        assert!(!voice.same_profile(Some(&profile(1.0))));
    }

    #[test]
    fn the_same_profile_is_the_same_by_reference_or_by_content() {
        let first = profile(1.0);
        let voice = WindVoice { loaded_for: Some(first.clone()), loops: Vec::new() };
        assert!(voice.same_profile(Some(&first)));
        assert!(voice.same_profile(Some(&profile(1.0))), "a profile read again is the same");
        assert!(!voice.same_profile(Some(&profile(0.5))));
        assert!(!voice.same_profile(None));
    }
}
