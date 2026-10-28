//! The start sound of a mod's engine switch (`bb.engine`, `docs/modding/api-engine.md`): a WAV of the mod's `files/` layers,
//! played once when the engine starts. A file that cannot be read is logged and the start is silent: a start never fails
//! the engine.

use kira::sound::static_sound::StaticSoundData;

use super::own_engine::parse_wav;
use crate::audio::volume::decibels;
use crate::audio::{Audio, Group, pcm};
use crate::engine_switch::StartSound;

/// Read the WAV `sound` names and play it once at its gain.
pub(super) fn play(audio: &mut Audio, sound: &StartSound) {
    let data = match read(audio, &sound.file) {
        Ok(data) => data,
        Err(error) => {
            log::warn!("the engine start sound {} does not play: {error}", sound.file);
            return;
        }
    };
    if let Err(error) = audio.play(Group::Sfx, data.volume(decibels(sound.gain))) {
        log::warn!("the engine start sound {} does not play: {error}", sound.file);
    }
}

/// The WAV at `file` of the install's layers, decoded as a one-shot.
fn read(audio: &Audio, file: &str) -> Result<StaticSoundData, String> {
    let bytes = audio.dir.read(file).map_err(|error| error.to_string())?;
    let mut wav = parse_wav(&bytes)?;
    wav.loop_range = None;
    Ok(pcm::sound(&wav))
}
