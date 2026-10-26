//! The `[engine_sound]` and `[car_sound]` tables of a car manifest: the car's own engine loops, and its own recordings of
//! the other sounds a car makes (shifts, turbo, backfire, pops). A car plays nothing of any other car: a sound it does not
//! give is silent, and an engine it does not give is the game's built-in one.
//!
//! `[engine_sound]`:
//!
//! The game plays every layer as a loop at once and mixes them by the engine's speed: a layer is loudest when the engine
//! turns at the layer's own `rpm` and fades out towards its neighbours, and each layer is sped up or slowed down in
//! proportion to the engine's rpm over its own. The files are 16-bit PCM WAV, mono or stereo, in the mod's `files/`
//! folder under the install's paths (`SOUND/...`).

use serde::Deserialize;

use super::ContentError;

/// Most layers a car may have.
pub const MAX_LAYERS: usize = 8;
/// Most files one list of `[car_sound]` may name.
pub const MAX_EFFECT_FILES: usize = 16;

/// One looping recording of the engine.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// The file, a path of the install's layout (`SOUND/MODENGINE/MYCAR/IDLE.WAV`).
    pub file: String,
    /// The engine speed the recording was made at.
    pub rpm: f32,
}

/// A car's own engine sound.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineSound {
    /// The rpm the engine idles at, and the rpm its rev limiter holds: what the game's `0 to 1` engine speed spans.
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    /// The layers, slowest first.
    pub layers: Vec<Layer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    idle_rpm: f32,
    redline_rpm: f32,
    layers: Vec<RawLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayer {
    file: String,
    rpm: f32,
}

const RPM_RANGE: std::ops::RangeInclusive<f32> = 100.0..=20_000.0;

/// Check the table. Layers are sorted by rpm; two layers at one rpm are refused.
pub(super) fn check(raw: Option<Raw>) -> Result<Option<EngineSound>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    let field = |field: &'static str, reason: &'static str| ContentError::Field { field, reason };
    if !RPM_RANGE.contains(&raw.idle_rpm) || !RPM_RANGE.contains(&raw.redline_rpm) {
        return Err(field("engine_sound.idle_rpm", "idle_rpm and redline_rpm are numbers from 100 to 20000"));
    }
    if raw.redline_rpm <= raw.idle_rpm {
        return Err(field("engine_sound.redline_rpm", "the redline is above the idle"));
    }
    if raw.layers.is_empty() || raw.layers.len() > MAX_LAYERS {
        return Err(field("engine_sound.layers", "a car has 1 to 8 layers"));
    }
    let mut layers = Vec::with_capacity(raw.layers.len());
    for layer in raw.layers {
        if !RPM_RANGE.contains(&layer.rpm) {
            return Err(field("engine_sound.layers.rpm", "a layer's rpm is a number from 100 to 20000"));
        }
        if !is_sound_path(&layer.file) {
            return Err(ContentError::InvalidText(layer.file));
        }
        layers.push(Layer { file: layer.file, rpm: layer.rpm });
    }
    layers.sort_by(|a, b| a.rpm.total_cmp(&b.rpm));
    if layers.windows(2).any(|pair| pair[0].rpm == pair[1].rpm) {
        return Err(field("engine_sound.layers.rpm", "two layers have the same rpm"));
    }
    Ok(Some(EngineSound { idle_rpm: raw.idle_rpm, redline_rpm: raw.redline_rpm, layers }))
}

/// A relative path of the install's layout that names a `.WAV` file: letters, digits, `_`, `-`, `.` and `/`, no `..`.
pub(super) fn is_sound_path(path: &str) -> bool {
    let safe = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    safe && parts_ok && path.len() <= 96 && path.to_ascii_lowercase().ends_with(".wav")
}

/// The car's own recordings of the sounds besides its engine (`[car_sound]`). Every list is optional; a list left out is
/// silence for that effect. The files are 16-bit PCM WAV, mono or stereo, in the mod's `files/` folder under the
/// install's paths (`SOUND/...`). A list with several files is variants: one is picked at random each time.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CarEffects {
    /// Played at every gear change that `shift_up` or `shift_down` does not cover.
    pub shift: Vec<String>,
    /// Played at an up shift. Falls back to `shift`.
    pub shift_up: Vec<String>,
    /// Played at a down shift. Falls back to `shift`.
    pub shift_down: Vec<String>,
    /// Loops of the turbocharger's whistle, quietest spool first: they are mixed by how far the turbo has spooled, and
    /// sped up as it does.
    pub turbo_whine: Vec<String>,
    /// Played when the throttle is lifted with the turbo spooled.
    pub turbo_blowoff: Vec<String>,
    /// Played when the throttle is lifted at high engine speed.
    pub backfire: Vec<String>,
    /// Short bangs that follow a backfire.
    pub pops: Vec<String>,
    /// The recordings every car of a mod shares (`[car_sound.shared]`): body hits, scrapes and pops.
    pub shared: super::shared_sound::SharedSound,
}

impl CarEffects {
    /// Whether no list names a file.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct RawEffects {
    #[serde(default)]
    shift: Vec<String>,
    #[serde(default)]
    shift_up: Vec<String>,
    #[serde(default)]
    shift_down: Vec<String>,
    #[serde(default)]
    turbo_whine: Vec<String>,
    #[serde(default)]
    turbo_blowoff: Vec<String>,
    #[serde(default)]
    backfire: Vec<String>,
    #[serde(default)]
    pops: Vec<String>,
    #[serde(default)]
    shared: super::shared_sound::RawShared,
}

/// Check the table. A table that names no file is the same as no table.
pub(super) fn check_effects(raw: Option<RawEffects>) -> Result<Option<CarEffects>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    let list = |name: &'static str, files: Vec<String>| -> Result<Vec<String>, ContentError> {
        if files.len() > MAX_EFFECT_FILES {
            return Err(ContentError::Field { field: name, reason: "a list names at most 16 files" });
        }
        if let Some(bad) = files.iter().find(|file| !is_sound_path(file)) {
            return Err(ContentError::InvalidText(bad.clone()));
        }
        Ok(files)
    };
    let effects = CarEffects {
        shift: list("car_sound.shift", raw.shift)?,
        shift_up: list("car_sound.shift_up", raw.shift_up)?,
        shift_down: list("car_sound.shift_down", raw.shift_down)?,
        turbo_whine: list("car_sound.turbo_whine", raw.turbo_whine)?,
        turbo_blowoff: list("car_sound.turbo_blowoff", raw.turbo_blowoff)?,
        backfire: list("car_sound.backfire", raw.backfire)?,
        pops: list("car_sound.pops", raw.pops)?,
        shared: super::shared_sound::check(raw.shared)?,
    };
    Ok((!effects.is_empty()).then_some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct File {
        car_sound: Option<RawEffects>,
    }

    fn effects(text: &str) -> Result<Option<CarEffects>, ContentError> {
        let file: File = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
        check_effects(file.car_sound)
    }

    #[test]
    fn a_car_has_no_effects_unless_it_names_a_file() {
        assert_eq!(effects("").unwrap(), None);
        assert_eq!(effects("[car_sound]\nshift = []\n").unwrap(), None);
    }

    #[test]
    fn every_list_is_read_and_a_list_left_out_is_empty() {
        let text = "[car_sound]\nshift = [\"SOUND/A/S0.WAV\", \"SOUND/A/S1.WAV\"]\nturbo_whine = [\"SOUND/A/T.WAV\"]\n\
                    backfire = [\"SOUND/A/B.WAV\"]\npops = [\"SOUND/A/P.WAV\"]\nturbo_blowoff = [\"SOUND/A/O.WAV\"]\n\
                    shift_up = [\"SOUND/A/U.WAV\"]\nshift_down = [\"SOUND/A/D.WAV\"]\n";
        let got = effects(text).unwrap().unwrap();
        assert_eq!(got.shift, ["SOUND/A/S0.WAV", "SOUND/A/S1.WAV"]);
        assert_eq!((got.shift_up[0].as_str(), got.shift_down[0].as_str()), ("SOUND/A/U.WAV", "SOUND/A/D.WAV"));
        assert_eq!(got.turbo_whine, ["SOUND/A/T.WAV"]);
        assert_eq!((got.backfire.len(), got.pops.len(), got.turbo_blowoff.len()), (1, 1, 1));
        let only = effects("[car_sound]\nbackfire = [\"SOUND/A/B.WAV\"]\n").unwrap().unwrap();
        assert!(only.shift.is_empty() && only.pops.is_empty() && !only.backfire.is_empty());
    }

    #[test]
    fn bad_effects_are_refused() {
        let seventeen = format!("[car_sound]\npops = [{}]\n", vec!["\"SOUND/P.WAV\""; 17].join(", "));
        for (edit, text) in [
            ("a path with ..", "[car_sound]\nshift = [\"SOUND/../S.WAV\"]\n"),
            ("a file that is not a WAV", "[car_sound]\npops = [\"SOUND/P.MP3\"]\n"),
            ("an absolute path", "[car_sound]\nbackfire = [\"/SOUND/B.WAV\"]\n"),
            ("seventeen files", seventeen.as_str()),
            ("a key the game does not know", "[car_sound]\nhorn = [\"SOUND/H.WAV\"]\n"),
            ("a file instead of a list", "[car_sound]\nshift = \"SOUND/S.WAV\"\n"),
        ] {
            assert!(effects(text).is_err(), "{edit}");
        }
    }
}
