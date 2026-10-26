//! The `[engine_sound]` table of a car manifest: the car's own engine loops.
//!
//! The game plays every layer as a loop at once and mixes them by the engine's speed: a layer is loudest when the engine
//! turns at the layer's own `rpm` and fades out towards its neighbours, and each layer is sped up or slowed down in
//! proportion to the engine's rpm over its own. The files are 16-bit PCM WAV, mono or stereo, in the mod's `files/`
//! folder under the install's paths (`SOUND/...`).

use serde::Deserialize;

use super::ContentError;

/// Most layers a car may have.
pub const MAX_LAYERS: usize = 8;

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
fn is_sound_path(path: &str) -> bool {
    let safe = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    safe && parts_ok && path.len() <= 96 && path.to_ascii_lowercase().ends_with(".wav")
}
