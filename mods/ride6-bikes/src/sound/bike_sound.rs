//! One bike's sound from the user's extracted banks: its engine layers from the two REV models, its effects from the
//! shift, backfire and exhaust events, written as shared files and a manifest text (`docs/modding/ride6-bikes-sound.md`).
//!
//! Files are shared between bikes. Engine layers go to `SOUND/RIDE6/ENG/<engine>-<hash>/`, named by the engine event and
//! the media of its two models, so bikes with one engine bank write them once; every effect recording goes to
//! `SOUND/RIDE6/FX/<media id>.WAV`, so a recording that many bikes name (the gear-up clunks of the shared vehicle bank,
//! for one) is decoded once. A bike's manifest tables name those paths.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::data::{BikeAudioData, Event};
use super::decode::Vgmstream;
use super::hirc::SoundRef;
use super::rev::{self, RevModel};
use super::rev_layers::{self, EngineLayers, Recordings};
use super::toml_out::{self, EffectLists, LayerEntry};
use super::wav::{self, Pcm};
use super::wwise::Wwise;

/// The peak of the loudest engine layer and of every one-shot (modelling choices, `docs/specs/ride6-engine-sound.md`).
const ENGINE_PEAK: f32 = 0.9;
const SHOT_PEAK: f32 = 0.9;
const MAX_LIST: usize = 16;

/// What was made for one bike.
#[derive(Debug, Clone, PartialEq)]
pub struct BikeSoundOut {
    /// The `[engine_sound]` and `[car_sound]` tables.
    pub tables: String,
    /// The folder of the engine layers, `SOUND/RIDE6/ENG/<key>`.
    pub engine_key: String,
    pub layers: usize,
    pub worst_detune: f32,
    pub effects: [usize; 4],
    pub notes: Vec<String>,
}

/// The state of a run: where the extracted banks are, the decoder and the output folder.
pub struct Builder {
    pub wwise: Wwise,
    pub vgmstream: Vgmstream,
    /// The output's `files` folder.
    pub files: PathBuf,
}

fn fnv1a(parts: &[u32]) -> u32 {
    parts
        .iter()
        .flat_map(|part| part.to_le_bytes())
        .fold(0x811C_9DC5_u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

/// A folder name for an engine: the event's name without its `_EN`, in letters, digits and `_`, then 8 hex digits of the
/// media ids.
pub fn engine_key(event_name: &str, media: [u32; 2]) -> String {
    let clean: String =
        event_name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let stem = clean.strip_suffix("_EN").or_else(|| clean.strip_suffix("_en")).unwrap_or(&clean);
    let head: String = stem.chars().take(30).collect();
    format!("{head}-{:08X}", fnv1a(&media))
}

/// `L<index>_<rpm in tenths>.WAV`, the file name of a layer; the rpm is read back from it when the set already exists.
fn layer_name(index: usize, rpm: f32) -> String {
    format!("L{index:02}_{}.WAV", (rpm * 10.0).round() as u32)
}

fn parse_layer_name(name: &str) -> Option<(usize, f32)> {
    let stem = name.strip_prefix('L')?.strip_suffix(".WAV")?;
    let (index, tenths) = stem.split_once('_')?;
    Some((index.parse().ok()?, tenths.parse::<f32>().ok()? / 10.0))
}

struct Model<'a> {
    vgmstream: &'a Vgmstream,
    file: PathBuf,
}

impl Recordings for Model<'_> {
    fn decode(&self, number: usize) -> Result<Pcm> {
        self.vgmstream.decode_file(&self.file, Some(number))
    }
}

impl Builder {
    /// A REV model of an event: its media id, the parsed directory, its bytes, and the sounds of the event.
    fn model(&mut self, event: &Event, tag: &str) -> Result<(u32, RevModel, Vec<u8>, Vec<SoundRef>)> {
        let (held_in, sounds) = self.wwise.event_sounds(event)?;
        let media = self.wwise.plugin_media(&held_in, &sounds, rev::PLUGIN_ID)?;
        let bytes = self.wwise.media(&held_in, media)?;
        let parsed = rev::parse(&bytes).with_context(|| format!("the {tag} model {media}"))?;
        Ok((media, parsed, bytes, sounds))
    }

    /// Write a model's bytes to the scratch folder, where `vgmstream-cli` reads them.
    fn stage(&self, tag: &str, media: u32, bytes: &[u8]) -> Result<PathBuf> {
        let file = self.vgmstream.work_dir().join(format!("{}-{tag}-{media}.wem", std::process::id()));
        fs::write(&file, bytes).with_context(|| format!("writing {}", file.display()))?;
        Ok(file)
    }

    /// The layers of an engine already written, or `None`.
    fn existing_layers(&self, key: &str) -> Option<Vec<LayerEntry>> {
        let dir = self.files.join("SOUND/RIDE6/ENG").join(key);
        let mut found = Vec::new();
        for entry in fs::read_dir(&dir).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let (index, rpm) = parse_layer_name(&name)?;
            found.push((index, LayerEntry { file: format!("SOUND/RIDE6/ENG/{key}/{name}"), rpm }));
        }
        found.sort_by_key(|(index, _)| *index);
        (!found.is_empty()).then(|| found.into_iter().map(|(_, entry)| entry).collect())
    }

    fn write_layers(&self, key: &str, mut layers: EngineLayers) -> Result<Vec<LayerEntry>> {
        rev_layers::normalise(&mut layers, ENGINE_PEAK)?;
        let dir = self.files.join("SOUND/RIDE6/ENG").join(key);
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let mut entries = Vec::new();
        for (index, layer) in layers.layers.iter().enumerate() {
            let name = layer_name(index, layer.rpm);
            fs::write(dir.join(&name), rev_layers::encode(layer)).with_context(|| format!("writing {name}"))?;
            entries.push(LayerEntry { file: format!("SOUND/RIDE6/ENG/{key}/{name}"), rpm: layer.rpm });
        }
        Ok(entries)
    }

    /// One effect recording as `SOUND/RIDE6/FX/<id>.WAV`, decoded and levelled once.
    fn effect(&mut self, bank_file: &str, id: u32) -> Result<String> {
        let name = format!("SOUND/RIDE6/FX/{id}.WAV");
        let target = self.files.join(&name);
        if target.is_file() {
            return Ok(name);
        }
        let bytes = self.wwise.media(bank_file, id)?;
        let mut pcm = self.vgmstream.decode_bytes(&id.to_string(), &bytes, None)?;
        pcm.loop_range = None;
        pcm.normalize(SHOT_PEAK);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&target, wav::encode16(&pcm)).with_context(|| format!("writing {}", target.display()))?;
        Ok(name)
    }

    /// The recordings of a list of media, skipping (and noting) those that cannot be read.
    fn effect_list(&mut self, what: &str, bank_file: &str, media: &[u32], notes: &mut Vec<String>) -> Vec<String> {
        let mut out = Vec::new();
        for &id in media.iter().take(MAX_LIST) {
            match self.effect(bank_file, id) {
                Ok(path) => out.push(path),
                Err(error) => notes.push(format!("{what}: media {id} left out: {error:#}")),
            }
        }
        out
    }

    /// The codec media an event plays and the bank file to read them from.
    fn event_media(&mut self, event: Option<&Event>) -> Result<(String, Vec<u32>)> {
        let Some(event) = event else { return Ok((String::new(), Vec::new())) };
        let (bank, sounds) = self.wwise.event_sounds(event)?;
        Ok((bank, Wwise::codec_media(&sounds)))
    }

    /// Build one bike's sound. `idle_rpm` and `redline_rpm` are the engine's own (the physics data).
    pub fn bike(&mut self, data: &BikeAudioData, idle_rpm: f32, redline_rpm: f32) -> Result<BikeSoundOut> {
        let (Some(en_event), Some(ex_event)) = (&data.events.engine_event, &data.events.exhaust_event) else {
            bail!("the bike names no engine or no exhaust event");
        };
        let Some(ex_bank) = ex_event.banks.first() else { bail!("the exhaust event names no bank") };
        let mut notes = Vec::new();
        let (en_media, en_model, en_bytes, _) = self.model(en_event, "engine")?;
        let (ex_media, ex_model, ex_bytes, ex_sounds) = self.model(ex_event, "exhaust")?;
        let key = engine_key(&en_event.name, [en_media, ex_media]);
        let (entries, worst_detune) = match self.existing_layers(&key) {
            Some(entries) => (entries, 0.0),
            None => {
                let (en_file, ex_file) =
                    (self.stage("engine", en_media, &en_bytes)?, self.stage("exhaust", ex_media, &ex_bytes)?);
                let en_audio = Model { vgmstream: &self.vgmstream, file: en_file.clone() };
                let ex_audio = Model { vgmstream: &self.vgmstream, file: ex_file.clone() };
                let built = rev_layers::build(
                    &en_model,
                    &en_audio,
                    &ex_model,
                    &ex_audio,
                    rev_layers::LOOP_LAYERS,
                    Some(idle_rpm),
                );
                let _ = (fs::remove_file(&en_file), fs::remove_file(&ex_file));
                let layers = built?;
                notes.extend(layers.notes.iter().cloned());
                let detune = layers.worst_detune;
                (self.write_layers(&key, layers)?, detune)
            }
        };
        let (up_bank, up) = self.event_media(data.events.gear_up_change_event.as_ref())?;
        let (down_bank, down) = self.event_media(data.events.gear_down_change_event.as_ref())?;
        let (bang_bank, bang) = self.event_media(data.events.backfire_bang_event.as_ref())?;
        let pops = Wwise::codec_media(&ex_sounds);
        let lists = EffectLists {
            shift_up: self.effect_list("shift_up", &up_bank, &up, &mut notes),
            shift_down: self.effect_list("shift_down", &down_bank, &down, &mut notes),
            backfire: self.effect_list("backfire", &bang_bank, &bang, &mut notes),
            pops: self.effect_list("pops", &ex_bank.file, &pops, &mut notes),
            shift_clunk: Some(false),
            shift_on_gear: true,
        };
        let overrun = data.overrun()?;
        let overrun = overrun.applies().then_some(overrun);
        let effects = [lists.shift_up.len(), lists.shift_down.len(), lists.backfire.len(), lists.pops.len()];
        let mut tables = toml_out::engine_sound(idle_rpm, redline_rpm, &entries);
        let car = toml_out::car_sound(&lists, overrun.as_ref());
        if !car.is_empty() {
            tables.push('\n');
            tables.push_str(&car);
        }
        Ok(BikeSoundOut { tables, engine_key: key, layers: entries.len(), worst_detune, effects, notes })
    }
}

/// The `Stock`/`Custom` resolved file of a chassis in `dir` (`<chassis>__<variant>.json`).
pub fn resolved_file(dir: &Path, chassis: &str, variant: &str) -> PathBuf {
    dir.join(format!("{chassis}__{variant}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_key_names_the_event_and_the_two_models() {
        let key = engine_key("KTM_EXC_450_EN", [435_470_671, 666_937_756]);
        assert!(key.starts_with("KTM_EXC_450-") && key.len() == "KTM_EXC_450-".len() + 8);
        assert_ne!(key, engine_key("KTM_EXC_450_EN", [435_470_671, 666_937_757]));
        assert_eq!(key, engine_key("KTM_EXC_450_EN", [435_470_671, 666_937_756]));
        let odd = engine_key("A b-c", [1, 2]);
        assert!(odd.starts_with("A_b_c-"));
        assert!(engine_key(&"X".repeat(80), [1, 2]).len() <= 39);
    }

    #[test]
    fn a_layer_name_carries_its_order_and_its_rpm() {
        assert_eq!(layer_name(3, 7243.4), "L03_72434.WAV");
        assert_eq!(parse_layer_name("L03_72434.WAV"), Some((3, 7243.4)));
        assert_eq!(parse_layer_name("readme.txt"), None);
        assert_eq!(parse_layer_name("L1_x.WAV"), None);
    }

    #[test]
    fn the_resolved_file_name_joins_chassis_and_variant() {
        assert_eq!(
            resolved_file(Path::new("d"), "KTM_450-SMR-2022", "Custom"),
            Path::new("d").join("KTM_450-SMR-2022__Custom.json")
        );
    }
}
