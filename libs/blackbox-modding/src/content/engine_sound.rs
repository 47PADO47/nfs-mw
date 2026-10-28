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
use super::engine_curves::{self, Curve, LayerKind, Pitch, RawCurve, RawPitch};

/// Most layers a car may have.
pub const MAX_LAYERS: usize = 12;
/// Most files one list of `[car_sound]` may name.
pub const MAX_EFFECT_FILES: usize = 16;

/// One layer of the engine: a looping recording, or a granular engine recording (`kind = "ginsu"`).
///
/// A layer without `gain` curves is mixed by engine speed: it is loudest at `rpm`, the speed it was recorded at. A layer
/// with `gain` curves plays by them (docs/modding/car-sound.md, "Curves") and its `rpm` is 0.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// The file, a path of the install's layout (`SOUND/MODENGINE/MYCAR/IDLE.WAV`, or `.GIN` for a Ginsu layer).
    pub file: String,
    /// The engine speed the recording was made at (0 for a layer with curves).
    pub rpm: f32,
    pub kind: LayerKind,
    /// The curves that multiply into the layer's volume; empty for a layer mixed by `rpm`.
    pub gain: Vec<Curve>,
    /// What the layer's playback rate follows; a layer without it plays at its own speed (curve layers) or at the
    /// engine's rpm over its own (layers mixed by `rpm`).
    pub pitch: Option<Pitch>,
    /// A curve of an input that is the layer's playback rate itself (1 plays the recording at its own speed), for a pitch
    /// that is not proportional to an input. It takes the place of `pitch`. Only a layer with `gain` curves has one.
    pub rate: Option<Curve>,
    /// For a Ginsu layer: the frequencies its recording is played at from idle to redline.
    pub span: Option<(f32, f32)>,
    /// A fixed level the curves are scaled by.
    pub volume: f32,
}

impl Layer {
    /// A layer mixed by the engine speed it was recorded at.
    pub fn classic(file: impl Into<String>, rpm: f32) -> Self {
        Self {
            file: file.into(),
            rpm,
            kind: LayerKind::Loop,
            gain: Vec::new(),
            pitch: None,
            rate: None,
            span: None,
            volume: 1.0,
        }
    }
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
    #[serde(default)]
    rpm: Option<f32>,
    #[serde(default)]
    kind: LayerKind,
    #[serde(default)]
    gain: Vec<RawCurve>,
    #[serde(default)]
    pitch: Option<RawPitch>,
    #[serde(default)]
    rate: Option<RawCurve>,
    #[serde(default)]
    span: Option<[f32; 2]>,
    #[serde(default)]
    volume: Option<f32>,
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
        layers.push(check_layer(layer)?);
    }
    layers.sort_by(|a, b| a.rpm.total_cmp(&b.rpm));
    let mixed = || layers.iter().filter(|layer| layer.gain.is_empty());
    if mixed().zip(mixed().skip(1)).any(|(a, b)| a.rpm == b.rpm) {
        return Err(field("engine_sound.layers.rpm", "two layers have the same rpm"));
    }
    Ok(Some(EngineSound { idle_rpm: raw.idle_rpm, redline_rpm: raw.redline_rpm, layers }))
}

/// Check one layer: its file, and either an `rpm` or curves, and the Ginsu span.
fn check_layer(layer: RawLayer) -> Result<Layer, ContentError> {
    let field = |field: &'static str, reason: &'static str| ContentError::Field { field, reason };
    let extension = if layer.kind == LayerKind::Ginsu { ".gin" } else { ".wav" };
    if !is_path(&layer.file, extension) {
        return Err(ContentError::InvalidText(layer.file));
    }
    let gain = engine_curves::check_curves(layer.gain)?;
    let rpm = match (layer.rpm, gain.is_empty()) {
        (Some(rpm), _) if !RPM_RANGE.contains(&rpm) => {
            return Err(field("engine_sound.layers.rpm", "a layer's rpm is a number from 100 to 20000"));
        }
        (Some(rpm), true) => rpm,
        (None, true) => return Err(field("engine_sound.layers.rpm", "a layer with no curves needs an rpm")),
        (_, false) => 0.0,
    };
    let span = match (layer.kind, layer.span) {
        (LayerKind::Ginsu, Some([low, high]))
            if RPM_RANGE.contains(&low) && RPM_RANGE.contains(&high) && low < high =>
        {
            Some((low, high))
        }
        (LayerKind::Ginsu, _) => {
            return Err(field(
                "engine_sound.layers.span",
                "a Ginsu layer has a span of two rising frequencies, 100 to 20000",
            ));
        }
        (LayerKind::Loop, None) => None,
        (LayerKind::Loop, Some(_)) => return Err(field("engine_sound.layers.span", "only a Ginsu layer has a span")),
    };
    if layer.kind == LayerKind::Ginsu && gain.is_empty() {
        return Err(field("engine_sound.layers.gain", "a Ginsu layer has curves"));
    }
    let volume = layer.volume.unwrap_or(1.0);
    if !(0.0..=engine_curves::MAX_LEVEL).contains(&volume) {
        return Err(field("engine_sound.layers.volume", "a layer's volume is a number from 0 to 8"));
    }
    let pitch = engine_curves::check_pitch(layer.pitch)?;
    let rate = match layer.rate {
        None => None,
        Some(_) if gain.is_empty() => {
            return Err(field("engine_sound.layers.rate", "a rate curve needs gain curves"));
        }
        Some(raw) => engine_curves::check_curves(vec![raw])?.into_iter().next(),
    };
    Ok(Layer { file: layer.file, rpm, kind: layer.kind, gain, pitch, rate, span, volume })
}

/// A relative path of the install's layout that names a `.WAV` file: letters, digits, `_`, `-`, `.` and `/`, no `..`.
/// A relative path of the install's layout that names a `.PNG` file, under the same rules as a sound path.
pub(super) fn is_png_path(path: &str) -> bool {
    let safe = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    safe && parts_ok && path.len() <= 96 && path.to_ascii_lowercase().ends_with(".png")
}

pub(super) fn is_sound_path(path: &str) -> bool {
    is_path(path, ".wav")
}

/// A relative path of the install's layout that ends in `extension` (lower case, with the dot).
fn is_path(path: &str, extension: &str) -> bool {
    let safe = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    safe && parts_ok && path.len() <= 96 && path.to_ascii_lowercase().ends_with(extension)
}

/// The car's own recordings of the sounds besides its engine (`[car_sound]`). Every list is optional; a list left out is
/// silence for that effect. The files are 16-bit PCM WAV, mono or stereo, in the mod's `files/` folder under the
/// install's paths (`SOUND/...`). A list with several files is variants: one is picked at random each time.
#[derive(Debug, Clone, PartialEq, Default)]
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
    /// `shift_clunk = false`: the car makes no gear clunk of the game's class-wide bank when it has no `shift`
    /// recording of its own (an electric car has no gears to clunk). By default a car without `shift` clunks.
    pub no_clunk: bool,
    /// The recordings every car of a mod shares (`[car_sound.shared]`): body hits, scrapes and pops.
    pub shared: super::shared_sound::SharedSound,
    /// `shift_trigger = "gear"`: the shift recordings play at every change of the gear number (up or down by comparing
    /// the numbers, no minimum engine speed) instead of when the engine mixer asks for a gear clunk.
    pub shift_on_gear: bool,
    /// `[car_sound.overrun]`: the accumulator backfire, which replaces the core backfire rule when present.
    pub overrun: Option<super::overrun_sound::Overrun>,
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
    shift_clunk: Option<bool>,
    #[serde(default)]
    shared: super::shared_sound::RawShared,
    #[serde(default)]
    shift_trigger: Option<ShiftTrigger>,
    #[serde(default)]
    overrun: Option<super::overrun_sound::RawOverrun>,
}

/// What sets off the shift recordings (`shift_trigger`).
#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ShiftTrigger {
    /// The engine mixer's gear clunk (the default).
    Mixer,
    /// Every change of the gear number.
    Gear,
}

/// Check the table. A table that names no file and changes no behaviour is the same as no table.
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
        no_clunk: raw.shift_clunk == Some(false),
        shared: super::shared_sound::check(raw.shared)?,
        shift_on_gear: raw.shift_trigger == Some(ShiftTrigger::Gear),
        overrun: super::overrun_sound::check(raw.overrun)?,
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
    fn a_car_can_ask_for_no_gear_clunk_and_that_alone_is_a_table() {
        let quiet = effects("[car_sound]\nshift_clunk = false\n").unwrap().unwrap();
        assert!(quiet.no_clunk && quiet.shift.is_empty());
        assert_eq!(effects("[car_sound]\nshift_clunk = true\n").unwrap(), None, "the default changes nothing");
        let own = effects("[car_sound]\nshift = [\"SOUND/A/S.WAV\"]\n").unwrap().unwrap();
        assert!(!own.no_clunk);
    }

    #[test]
    fn the_shift_trigger_is_the_mixer_unless_the_gear_is_asked_for() {
        let table = |trigger: &str| format!("[car_sound]\nshift_trigger = \"{trigger}\"\n");
        assert_eq!(effects(&table("mixer")).unwrap(), None, "the default changes nothing");
        let on_gear = effects(&table("gear")).unwrap().unwrap();
        assert!(on_gear.shift_on_gear && on_gear.shift.is_empty());
        assert!(effects(&table("clutch")).is_err());
        let own = effects("[car_sound]\nshift_up = [\"SOUND/A/U.WAV\"]\n").unwrap().unwrap();
        assert!(!own.shift_on_gear);
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
