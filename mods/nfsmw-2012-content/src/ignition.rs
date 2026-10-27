//! The start-up sound of a car's engine, for the engine auto-stop (`engine_stop.rs`, `docs/modding/api-engine.md`).
//!
//! The 2012 car bundle holds a sound sequence named `Player Vehicle Ignition` (`chameleon-engine-state`): one sample, played
//! once. The sample is in the car's bundle or in the samples the cars share (`VEHICLES/WAVES.BNDL`), and 40 of the 93 cars
//! have such a sequence. Each car gets its own as `SOUND/MODENGINE/<ID>/IGNITION.WAV`; the cars without one, and the cars
//! of the 2005 game, use `SOUND/MODSHARED/ENGINE/IGNITION.WAV`, which is the ignition of the first car that has one
//! (a 2012 recording), or a short generated starter whirr when the install has none.

use std::path::Path;

use anyhow::{Context, Result};
use chameleon_bundle::Bundle;
use chameleon_engine_state::read_ignition;

use crate::sample::{self, wav_stereo_bytes};
use crate::source::Source;

/// The peak the sound is brought to, before the sequence's own gain.
const PEAK: f32 = 0.9;
/// A recording is not boosted more than this.
const MAX_BOOST: f32 = 8.0;
/// The shared file, relative to the mod's `files/` folder.
pub const SHARED_PATH: &str = "SOUND/MODSHARED/ENGINE/IGNITION.WAV";
/// The generated starter whirr, in seconds.
const GENERATED_SECONDS: f32 = 0.7;
const GENERATED_RATE: u32 = 48_000;

/// The path of a car's own ignition sound relative to the mod's `files/` folder.
pub fn car_path(id: &str) -> String {
    format!("SOUND/MODENGINE/{id}/IGNITION.WAV")
}

/// The ignition sound of the car in `bundle` as a stereo WAV, when the bundle has the sequence and its sample can be read.
/// `shared` is the shared sample bundle, where some cars keep it.
pub fn wav(bundle: &Bundle, shared: Option<&[u8]>) -> Option<Vec<u8>> {
    let ignition = read_ignition(bundle)?;
    let sound = match bundle.find(ignition.sample) {
        Some(at) => sample::read_stereo(bundle, at)?,
        None => {
            let shared = Bundle::parse(shared?).ok()?;
            let at = shared.find(ignition.sample)?;
            sample::read_stereo(&shared, at)?
        }
    };
    Some(shot(&sound.frames, sound.rate, ignition.gain))
}

/// A one-shot at a peak of [`PEAK`] times the sequence's gain (at most 1).
fn shot(frames: &[[f32; 2]], rate: u32, sequence_gain: f32) -> Vec<u8> {
    let loudest = frames.iter().flatten().fold(0.0f32, |peak, value| peak.max(value.abs()));
    let fit = match loudest > 1e-4 {
        true => (PEAK / loudest).min(MAX_BOOST),
        false => 1.0,
    };
    wav_stereo_bytes(frames, rate, fit * sequence_gain.clamp(0.1, 1.0))
}

/// Write the shared ignition sound into the mod folder `out`: the first car's of the install that has one, else the
/// generated whirr. A failure only costs the shared sound: the cars that have their own still start with it.
pub fn write_shared(source: &Source, out: &Path) {
    let from_install = source.cars().iter().find_map(|file| {
        let bytes = source.bundle_bytes(file).ok()?;
        let bundle = Bundle::parse(&bytes).ok()?;
        wav(&bundle, source.shared_samples())
    });
    if from_install.is_none() {
        log::warn!("no 2012 car has a readable ignition sound: the shared engine start is a generated whirr");
    }
    let bytes = from_install.unwrap_or_else(generated);
    if let Err(error) = write(out, &bytes) {
        log::warn!("the shared engine start sound is not written: {error:#}");
    }
}

fn write(out: &Path, bytes: &[u8]) -> Result<()> {
    let path = out.join("files").join(SHARED_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))
}

/// A short starter: a rough turning engine that speeds up while it cranks, catches at two thirds of the way and settles to
/// an idle. Made of harmonics of a rising fundamental and a little noise after the catch; the same bytes every time. It
/// stands in for a recording when the install gives none.
pub fn generated() -> Vec<u8> {
    let count = (GENERATED_SECONDS * GENERATED_RATE as f32) as usize;
    let catch = GENERATED_SECONDS * 0.65;
    let mut phase = 0.0f32;
    let mut noise = 0x2545_F491u32;
    let mut frames = Vec::with_capacity(count);
    for at in 0..count {
        let t = at as f32 / GENERATED_RATE as f32;
        // The cranking fundamental rises from 9 to 20 Hz, then the running engine idles at 26 Hz.
        let hz = match t < catch {
            true => 9.0 + 11.0 * t / catch,
            false => 26.0,
        };
        phase += std::f32::consts::TAU * hz / GENERATED_RATE as f32;
        let tone: f32 = (1..=6).map(|k| (k as f32 * phase).sin() / k as f32).sum();
        noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let rasp = (noise >> 16) as f32 / 32768.0 - 1.0;
        let level = match t < catch {
            true => 0.45 + 0.4 * t / catch,
            false => 0.85 * (-(t - catch) * 4.0).exp() + 0.3,
        };
        let fade = ((GENERATED_SECONDS - t) / 0.08).clamp(0.0, 1.0);
        let rasp_level = match t < catch {
            true => 0.0,
            false => 0.12,
        };
        let value = (tone * 0.55 + rasp * rasp_level) * level * fade;
        frames.push([value, value]);
    }
    shot(&frames, GENERATED_RATE, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames_of(wav: &[u8]) -> Vec<i16> {
        wav[44..].as_chunks::<2>().0.iter().map(|pair| i16::from_le_bytes(*pair)).collect()
    }

    #[test]
    fn a_cars_own_sound_is_in_its_folder() {
        assert_eq!(car_path("POR911CARR12"), "SOUND/MODENGINE/POR911CARR12/IGNITION.WAV");
    }

    #[test]
    fn the_generated_whirr_is_a_short_stereo_wav_with_a_peak_below_full_scale() {
        let wav = generated();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 2, "stereo");
        let samples = frames_of(&wav);
        assert_eq!(samples.len(), 2 * (GENERATED_SECONDS * GENERATED_RATE as f32) as usize);
        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > 20_000 && peak < 32_000, "peak {peak}");
        assert_eq!(wav, generated(), "the same bytes every time");
        assert!(samples.last().unwrap().abs() < 500, "it fades out");
    }

    #[test]
    fn a_shot_is_brought_to_the_peak_and_the_sequence_gain_only_lowers_it() {
        let frames = vec![[0.25, -0.25]; 4];
        let peak = |gain: f32| frames_of(&shot(&frames, 48_000, gain)).iter().map(|s| s.unsigned_abs()).max().unwrap();
        let full = peak(1.0);
        assert!((i32::from(full) - (PEAK * 32767.0) as i32).abs() < 4, "{full}");
        assert!(peak(0.5) < full);
        assert_eq!(peak(2.8), full, "a gain above 1 does not boost it");
    }

    #[test]
    fn silence_stays_silent() {
        assert!(frames_of(&shot(&[[0.0, 0.0]; 8], 48_000, 1.0)).iter().all(|s| *s == 0));
    }
}
