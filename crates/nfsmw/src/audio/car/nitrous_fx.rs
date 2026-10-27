//! The sounds of a mod's nitrous variant (`docs/modding/nitrous-variants.md`): a start one-shot when the nitrous lights, a
//! loop over the burn and an end one-shot when it stops, from the WAV files its profile names.
//!
//! The files are read from the install's layout (the mod's `files/` overlay), once per profile. When the profile asks to
//! replace the game's own nitrous sound ([`AudioProfile::replace_original`]) and its sounds loaded, [`NitrousVoice::update`]
//! answers true and the car's power effects keep silent about the nitrous. A profile whose files cannot be read leaves the
//! game's sound playing: a variant never makes the nitrous mute.

use std::sync::Arc;
use std::time::Duration;

use blackbox_modding::content::{AudioProfile, NitrousProfile};
use kira::Tween;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};

use super::own_engine::parse_wav;
use crate::audio::volume::decibels;
use crate::audio::{Audio, Group, pcm};

/// The sounds of the profile in use, decoded.
struct Sounds {
    profile: AudioProfile,
    start: Option<StaticSoundData>,
    looped: Option<StaticSoundData>,
    end: Option<StaticSoundData>,
}

/// The voice of the nitrous variant of one car.
#[derive(Default)]
pub struct NitrousVoice {
    /// The profile `sounds` were read for.
    loaded_for: Option<Arc<NitrousProfile>>,
    sounds: Option<Sounds>,
    burning: bool,
    looping: Option<StaticSoundHandle>,
}

impl NitrousVoice {
    /// One step: follow the profile (`fx`, the mod variant's, or `None`), and play its sounds at the edges of `burning`.
    /// Returns whether the profile's sounds stand in for the game's own nitrous sound.
    pub fn update(&mut self, audio: &mut Audio, fx: Option<&Arc<NitrousProfile>>, burning: bool) -> bool {
        let wanted = fx.filter(|profile| profile.audio.is_some());
        if !self.same_profile(wanted) {
            self.stop_loop(Duration::from_millis(120));
            self.sounds = wanted.and_then(|profile| load(audio, profile));
            self.loaded_for = wanted.cloned();
        }
        let was_burning = std::mem::replace(&mut self.burning, burning);
        if self.sounds.is_none() {
            return false;
        }
        if burning && !was_burning {
            self.light(audio);
        }
        if !burning && was_burning {
            self.put_out(audio);
        }
        self.sounds.as_ref().is_some_and(|sounds| sounds.profile.replace_original)
    }

    fn same_profile(&self, wanted: Option<&Arc<NitrousProfile>>) -> bool {
        match (&self.loaded_for, wanted) {
            (None, None) => true,
            (Some(have), Some(want)) => Arc::ptr_eq(have, want) || **have == **want,
            _ => false,
        }
    }

    /// The nitrous lit: the start one-shot, and the loop faded in.
    fn light(&mut self, audio: &mut Audio) {
        let Some(sounds) = self.sounds.as_ref() else { return };
        if let Some(start) = &sounds.start {
            let data = start.volume(decibels(sounds.profile.start_gain));
            if let Err(error) = audio.play(Group::Sfx, data) {
                log::warn!("the nitrous start sound does not play: {error}");
            }
        }
        let Some(looped) = &sounds.looped else { return };
        let fade = Tween { duration: seconds(sounds.profile.loop_fade_in), ..Tween::default() };
        let data = looped.volume(decibels(sounds.profile.loop_gain)).fade_in_tween(fade);
        match audio.play(Group::Sfx, data) {
            Ok(handle) => self.looping = Some(handle),
            Err(error) => log::warn!("the nitrous loop does not play: {error}"),
        }
    }

    /// The nitrous out: the loop faded away, and the end one-shot.
    fn put_out(&mut self, audio: &mut Audio) {
        let Some(sounds) = self.sounds.as_ref() else { return };
        let fade = seconds(sounds.profile.loop_fade_out);
        let end = sounds.end.as_ref().map(|end| end.volume(decibels(sounds.profile.end_gain)));
        self.stop_loop(fade);
        let Some(end) = end else { return };
        if let Err(error) = audio.play(Group::Sfx, end) {
            log::warn!("the nitrous end sound does not play: {error}");
        }
    }

    fn stop_loop(&mut self, fade: Duration) {
        let Some(mut handle) = self.looping.take() else { return };
        handle.stop(Tween { duration: fade, ..Tween::default() });
    }
}

impl Drop for NitrousVoice {
    /// A kira handle that is dropped leaves its sound playing, so the loop is stopped here.
    fn drop(&mut self) {
        self.stop_loop(Duration::from_millis(120));
    }
}

/// `value` seconds, never negative or non-finite.
fn seconds(value: f32) -> Duration {
    match value.is_finite() && value > 0.0 {
        true => Duration::from_secs_f32(value),
        false => Duration::ZERO,
    }
}

/// One WAV of the profile, decoded as a sound that plays once or loops over its whole length.
fn read_wav(audio: &Audio, file: &Option<String>, looped: bool) -> Result<Option<StaticSoundData>, String> {
    let Some(file) = file else { return Ok(None) };
    let bytes = audio.dir.read(file).map_err(|error| format!("{file}: {error}"))?;
    let mut wav = parse_wav(&bytes).map_err(|error| format!("{file}: {error}"))?;
    if !looped {
        wav.loop_range = None;
    }
    Ok(Some(pcm::sound(&wav)))
}

/// Read and decode the sounds the profile names. `None` when the profile has no audio or a named file cannot be read, so
/// the game's own sound stays.
fn load(audio: &Audio, profile: &NitrousProfile) -> Option<Sounds> {
    let audio_profile = profile.audio.clone()?;
    let loaded = (|| {
        Ok::<_, String>(Sounds {
            start: read_wav(audio, &audio_profile.start, false)?,
            looped: read_wav(audio, &audio_profile.looped, true)?,
            end: read_wav(audio, &audio_profile.end, false)?,
            profile: audio_profile.clone(),
        })
    })();
    match loaded {
        Ok(sounds) => Some(sounds),
        Err(error) => {
            log::warn!("the nitrous variant's sounds are not used, the game's own play: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fade_is_a_duration_and_a_bad_number_is_none() {
        assert_eq!(seconds(0.5), Duration::from_millis(500));
        assert_eq!(seconds(-1.0), Duration::ZERO);
        assert_eq!(seconds(f32::NAN), Duration::ZERO);
    }

    #[test]
    fn a_voice_that_has_no_profile_is_the_same_as_none() {
        let voice = NitrousVoice::default();
        assert!(voice.same_profile(None));
        let profile = Arc::new(NitrousProfile::default());
        assert!(!voice.same_profile(Some(&profile)));
    }
}
