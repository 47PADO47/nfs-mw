//! From the user's decoded recordings to the mod's sound files and the manifest tables of one bike.
//!
//! Nothing here knows where a recording came from: the caller names WAV files (decoded from the user's own install, see
//! `docs/modding/ride6-bikes-sound.md`) and the engine speed each engine loop was recorded at. The result is the set of
//! `files/CARS/<ID>/SOUND/*.WAV` and the text of `[engine_sound]` / `[car_sound]`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use super::data::OverrunParams;
use super::toml_out::{self, EffectLists, LayerEntry};
use super::wav::{self, Pcm};

/// The most layers a manifest takes, files per list, and characters of a path (`libs/blackbox-modding`).
const MAX_LAYERS: usize = 24;
const MAX_LIST: usize = 16;
const MAX_PATH: usize = 96;
const RPM_RANGE: std::ops::RangeInclusive<f32> = 100.0..=20_000.0;

/// One engine loop: the engine speed it was recorded at and the files that sound together at that speed (typically the
/// engine and the exhaust recording of the same speed, each with the gain the mix gives it).
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayerSource {
    pub rpm: f32,
    pub parts: Vec<(PathBuf, f32)>,
}

impl EngineLayerSource {
    /// Parse `"<rpm>:<file>[*gain],<file>[*gain]..."`, e.g. `"4500:en_4500.wav,ex_4500.wav*0.8"`.
    pub fn parse(text: &str) -> Result<Self> {
        let Some((rpm, files)) = text.split_once(':') else { bail!("`{text}`: expected `<rpm>:<file>[*gain],...`") };
        let rpm: f32 = rpm.trim().parse().with_context(|| format!("`{text}`: the rpm"))?;
        let mut parts = Vec::new();
        for item in files.split(',') {
            let (path, gain) = match item.rsplit_once('*') {
                Some((path, gain)) => (path, gain.trim().parse().with_context(|| format!("`{item}`: the gain"))?),
                None => (item, 1.0),
            };
            parts.push((PathBuf::from(path.trim()), gain));
        }
        ensure!(!parts.is_empty(), "`{text}`: no file");
        Ok(Self { rpm, parts })
    }
}

/// Everything the user supplied.
#[derive(Debug, Clone, Default)]
pub struct SoundSources {
    pub engine_layers: Vec<EngineLayerSource>,
    pub backfire: Vec<PathBuf>,
    pub shift_up: Vec<PathBuf>,
    pub shift_down: Vec<PathBuf>,
    pub pops: Vec<PathBuf>,
}

/// How to write them.
#[derive(Debug, Clone)]
pub struct SoundOptions {
    /// The car's folder name in the mod: letters, digits, `_` and `-` (`KTM450`).
    pub car_id: String,
    /// The engine's idle and limiter speed in rpm, from the bike's physics data.
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    /// Length of the crossfade that closes each engine loop, milliseconds (0 keeps the loop body as it is).
    pub loop_fade_ms: f32,
    /// The peak of the loudest engine loop (the others keep their level relative to it), 0 to 1.
    pub engine_peak: f32,
    /// The peak every one-shot is brought to, 0 to 1.
    pub shot_peak: f32,
    /// `Some(false)` writes `shift_clunk = false`.
    pub shift_clunk: Option<bool>,
    /// The overrun backfire table to write (`[car_sound.overrun]`); `None` leaves the backfire to the core rule.
    pub overrun: Option<OverrunParams>,
    /// Write `shift_trigger = "gear"`: the shift recordings play at every change of the gear number, as the original's
    /// gear events do (spec section 3), instead of at the engine mixer's gear clunk.
    pub shift_on_gear: bool,
}

impl SoundOptions {
    /// Defaults for a bike: 40 ms loop seams, engine peak 0.9, shot peak 0.9, no car clunk, shifts by the gear number, no
    /// overrun table (the caller sets it from the bike's data).
    pub fn new(car_id: impl Into<String>, idle_rpm: f32, redline_rpm: f32) -> Self {
        Self {
            car_id: car_id.into(),
            idle_rpm,
            redline_rpm,
            loop_fade_ms: 40.0,
            engine_peak: 0.9,
            shot_peak: 0.9,
            shift_clunk: Some(false),
            overrun: None,
            shift_on_gear: true,
        }
    }
}

/// A file to write under the mod's `files/` folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundFile {
    /// The path in the install's layout, also what the manifest names (`CARS/KTM450/SOUND/ENG0.WAV`).
    pub path: String,
    pub bytes: Vec<u8>,
}

/// The result: the files and the two manifest texts.
#[derive(Debug, Clone, PartialEq)]
pub struct BikeSound {
    pub files: Vec<SoundFile>,
    pub engine_sound_toml: String,
    /// Empty when no list is named and no clunk flag is set.
    pub car_sound_toml: String,
}

impl BikeSound {
    /// Both tables, separated by a blank line, for pasting after the car's own keys.
    pub fn manifest_text(&self) -> String {
        if self.car_sound_toml.is_empty() {
            return self.engine_sound_toml.clone();
        }
        format!("{}\n{}", self.engine_sound_toml, self.car_sound_toml)
    }
}

fn read_pcm(path: &Path) -> Result<Pcm> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    wav::parse(&bytes).with_context(|| format!("decoding {}", path.display()))
}

fn check_options(options: &SoundOptions) -> Result<()> {
    let id = &options.car_id;
    ensure!(
        !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        "car id `{id}`: letters, digits, `_` and `-` only"
    );
    ensure!(
        RPM_RANGE.contains(&options.idle_rpm) && RPM_RANGE.contains(&options.redline_rpm),
        "idle and redline are 100 to 20000 rpm"
    );
    ensure!(options.idle_rpm < options.redline_rpm, "the redline is above the idle");
    ensure!((0.0..=1.0).contains(&options.engine_peak) && (0.0..=1.0).contains(&options.shot_peak), "peaks are 0 to 1");
    Ok(())
}

fn sound_path(options: &SoundOptions, stem: &str, index: usize) -> Result<String> {
    let path = format!("CARS/{}/SOUND/{stem}{index}.WAV", options.car_id.to_ascii_uppercase());
    ensure!(path.len() <= MAX_PATH, "`{path}` is longer than {MAX_PATH} characters");
    Ok(path)
}

/// Decode, loop-fix and level the engine layers; the loudest one peaks at `engine_peak`.
fn engine_layers(sources: &SoundSources, options: &SoundOptions) -> Result<Vec<(f32, Pcm)>> {
    let layers = &sources.engine_layers;
    ensure!(!layers.is_empty() && layers.len() <= MAX_LAYERS, "an engine has 1 to {MAX_LAYERS} layers");
    let mut out = Vec::new();
    for layer in layers {
        ensure!(RPM_RANGE.contains(&layer.rpm), "layer rpm {} is outside 100 to 20000", layer.rpm);
        let mut decoded = Vec::new();
        for (path, gain) in &layer.parts {
            decoded.push((read_pcm(path)?.loop_body(), *gain));
        }
        let parts: Vec<(&Pcm, f32)> = decoded.iter().map(|(pcm, gain)| (pcm, *gain)).collect();
        let mixed = wav::mix_down(&parts)?;
        let fade = (mixed.sample_rate as f32 * options.loop_fade_ms / 1000.0) as usize;
        out.push((layer.rpm, mixed.seamless_loop(fade)));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    ensure!(out.windows(2).all(|pair| pair[0].0 != pair[1].0), "two layers have the same rpm");
    let loudest = out.iter().map(|(_, pcm)| pcm.peak()).fold(0.0_f32, f32::max);
    ensure!(loudest > 1e-6, "every engine layer is silent");
    let gain = options.engine_peak / loudest;
    for (_, pcm) in &mut out {
        pcm.samples.iter_mut().for_each(|sample| *sample *= gain);
    }
    Ok(out)
}

/// Decode a list of one-shots, each brought to the shot peak, and name them `<stem><n>.WAV`.
fn shots(paths: &[PathBuf], stem: &str, options: &SoundOptions, files: &mut Vec<SoundFile>) -> Result<Vec<String>> {
    ensure!(paths.len() <= MAX_LIST, "`{stem}`: at most {MAX_LIST} files");
    let mut names = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let mut pcm = read_pcm(path)?;
        pcm.loop_range = None;
        pcm.normalize(options.shot_peak);
        let name = sound_path(options, stem, index)?;
        files.push(SoundFile { path: name.clone(), bytes: wav::encode16(&pcm) });
        names.push(name);
    }
    Ok(names)
}

/// Build the sound of one bike.
pub fn build(sources: &SoundSources, options: &SoundOptions) -> Result<BikeSound> {
    check_options(options)?;
    let mut files = Vec::new();
    let mut entries = Vec::new();
    for (index, (rpm, pcm)) in engine_layers(sources, options)?.into_iter().enumerate() {
        let path = sound_path(options, "ENG", index)?;
        files.push(SoundFile { path: path.clone(), bytes: wav::encode16(&pcm) });
        entries.push(LayerEntry { file: path, rpm });
    }
    let lists = EffectLists {
        shift_up: shots(&sources.shift_up, "SHIFTUP", options, &mut files)?,
        shift_down: shots(&sources.shift_down, "SHIFTDN", options, &mut files)?,
        backfire: shots(&sources.backfire, "BACKFIRE", options, &mut files)?,
        pops: shots(&sources.pops, "POP", options, &mut files)?,
        shift_clunk: options.shift_clunk,
        shift_on_gear: options.shift_on_gear,
    };
    Ok(BikeSound {
        files,
        engine_sound_toml: toml_out::engine_sound(options.idle_rpm, options.redline_rpm, &entries),
        car_sound_toml: toml_out::car_sound(&lists, options.overrun.as_ref()),
    })
}

/// Write the files under `<mod folder>/files/`.
pub fn write_files(sound: &BikeSound, files_dir: &Path) -> Result<()> {
    for file in &sound.files {
        let target = files_dir.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&target, &file.bytes).with_context(|| format!("writing {}", target.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tone(dir: &Path, name: &str, freq: f32, amplitude: f32) -> PathBuf {
        let samples = (0..8000).map(|i| (i as f32 * freq).sin() * amplitude).collect();
        let pcm = Pcm { sample_rate: 8000, channels: 1, samples, loop_range: None };
        let path = dir.join(name);
        fs::write(&path, wav::encode16(&pcm)).unwrap();
        path
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-sound-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_bike_is_built_from_synthetic_recordings() {
        let dir = temp("build");
        let loud = write_tone(&dir, "high.wav", 0.3, 0.8);
        let quiet = write_tone(&dir, "low.wav", 0.1, 0.2);
        let bang = write_tone(&dir, "bang.wav", 0.7, 0.3);
        let sources = SoundSources {
            engine_layers: vec![
                EngineLayerSource { rpm: 7000.0, parts: vec![(loud, 1.0)] },
                EngineLayerSource { rpm: 3000.0, parts: vec![(quiet, 1.0)] },
            ],
            backfire: vec![bang.clone(), bang],
            ..SoundSources::default()
        };
        let options = SoundOptions::new("ktm450", 2300.0, 11000.0);
        let sound = build(&sources, &options).unwrap();
        assert_eq!(sound.files.len(), 4);
        assert_eq!(sound.files[0].path, "CARS/KTM450/SOUND/ENG0.WAV");
        // The slower layer comes first and the loudest peaks at 0.9.
        assert!(
            sound
                .engine_sound_toml
                .contains("rpm = 3000.0 },\n  { file = \"CARS/KTM450/SOUND/ENG1.WAV\", rpm = 7000.0")
        );
        let high = wav::parse(&sound.files[1].bytes).unwrap();
        assert!((high.peak() - 0.9).abs() < 0.01);
        assert!(
            sound
                .car_sound_toml
                .contains("backfire = [\"CARS/KTM450/SOUND/BACKFIRE0.WAV\", \"CARS/KTM450/SOUND/BACKFIRE1.WAV\"]")
        );
        assert!(sound.car_sound_toml.contains("shift_clunk = false"));
        assert!(sound.car_sound_toml.contains("shift_trigger = \"gear\""));
        let out = dir.join("files");
        write_files(&sound, &out).unwrap();
        assert!(out.join("CARS/KTM450/SOUND/ENG0.WAV").is_file());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bad_input_is_refused() {
        let dir = temp("bad");
        let tone = write_tone(&dir, "a.wav", 0.2, 0.5);
        let layer = |rpm| EngineLayerSource { rpm, parts: vec![(tone.clone(), 1.0)] };
        let build_with = |layers: Vec<EngineLayerSource>, id: &str| {
            let sources = SoundSources { engine_layers: layers, ..SoundSources::default() };
            build(&sources, &SoundOptions::new(id, 2000.0, 9000.0))
        };
        assert!(build_with(vec![], "X").is_err());
        assert!(build_with(vec![layer(3000.0), layer(3000.0)], "X").is_err());
        assert!(build_with(vec![layer(50.0)], "X").is_err());
        assert!(build_with(vec![layer(3000.0)], "bad id").is_err());
        assert!(build_with(vec![layer(3000.0)], "ok").is_ok());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_layer_spec_parses() {
        let layer = EngineLayerSource::parse("4500:en.wav,ex.wav*0.8").unwrap();
        assert_eq!(layer.rpm, 4500.0);
        assert_eq!(layer.parts, vec![(PathBuf::from("en.wav"), 1.0), (PathBuf::from("ex.wav"), 0.8)]);
        assert!(EngineLayerSource::parse("nope").is_err());
        assert!(EngineLayerSource::parse("x:a.wav").is_err());
    }
}
