//! Reading the `.gin` loops from the install: the tables from `blackbox-ginsu`, the audio from `ea-audio`.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use blackbox_ginsu::{GinsuData, GinsuTables};
use game_install::GameDir;

use super::{ENGINE_DIR, EngineSound};

/// The decoded Ginsu loops of one engine.
#[derive(Debug, Clone)]
pub struct EngineLoops {
    /// The accelerate loop; `None` when the engine set names none.
    pub accel: Option<Arc<GinsuData>>,
    /// The decelerate loop; `None` when the engine set names none.
    pub decel: Option<Arc<GinsuData>>,
}

/// Reads and decodes one `.gin` loop of `SOUND/ENGINE` (`name` as in `engineaudio.Filename_GinsuAccel`).
pub fn load_loop(dir: &GameDir, name: &str) -> Result<GinsuData> {
    let rel = format!("{ENGINE_DIR}/{name}");
    let bytes = dir.read(&rel).with_context(|| format!("reading {rel}"))?;
    let (tables, payload_at) = GinsuTables::parse(&bytes).with_context(|| format!("parsing the tables of {rel}"))?;
    let pcm = ea_audio::gin::decode(&bytes).with_context(|| format!("decoding {rel}"))?;
    if pcm.sample_rate != tables.sample_rate() {
        bail!("{rel}: the decoder reports {} Hz but the tables {} Hz", pcm.sample_rate, tables.sample_rate());
    }
    if bytes.len() - payload_at < tables.xas_payload_len() {
        bail!("{rel}: the audio data is shorter than the tables announce");
    }
    GinsuData::from_i16(tables, &pcm.samples).with_context(|| format!("pairing the tables of {rel} with its audio"))
}

impl EngineLoops {
    /// Loads the loops `engine` names.
    pub fn load(dir: &GameDir, engine: &EngineSound) -> Result<Self> {
        let load = |name: &str| match name.is_empty() {
            true => Ok(None),
            false => load_loop(dir, name).map(|data| Some(Arc::new(data))),
        };
        Ok(Self { accel: load(&engine.accel_loop)?, decel: load(&engine.decel_loop)? })
    }
}
