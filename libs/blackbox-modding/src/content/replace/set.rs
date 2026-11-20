//! `replace/<id>.toml`: one replacement set (see the module docs of [`super`]).

use std::collections::HashSet;

use serde::Deserialize;

use super::MAX_LABEL_BYTES;
use super::texture::{RawTexture, TextureEntry};
use crate::content::engine_sound::is_pack_path;
use crate::content::model_swap::{Fit as SwapFit, FitLimits, RawLimits, field, is_solid_name};
use crate::content::scenery::{Place, RawPlace, place};
use crate::content::{ContentError, variant};

/// The folder of a mod that holds one manifest per replacement set.
pub const REPLACE_FOLDER: &str = "replace";
/// The most entries of one kind a set may have: model targets, textures, or added objects.
pub const MAX_ENTRIES: usize = 4096;
/// The largest priority, up and down.
pub const MAX_PRIORITY: i32 = 1000;
/// The most objects a set may add.
const MAX_ADDS: usize = 65_536;
/// The longest kind and value ids, in bytes.
const MAX_ID_BYTES: usize = 32;

/// What a replacement is looked up by: the game's name for it, or the hash the game keys it by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// The name as the game spells it. The game hashes it its own way (the hash may be case-sensitive).
    Name(String),
    /// The hash itself, for a thing whose name is not known.
    Hash(u32),
}

/// One model of the set that takes the place of one or more models of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    /// The game's models it replaces: one or more, each once in the set.
    pub targets: Vec<Target>,
    /// The solid of the set's geometry file that takes their place.
    pub model: String,
    /// How it is fitted into what it replaces. The set's default when the entry gives none.
    pub fit: SwapFit,
}

/// A checked `replace/<id>.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceSet {
    /// The set's id: the file name without `.toml`.
    pub id: String,
    /// The kind the set belongs to (the option group); empty for an `always` set that names none.
    pub kind: String,
    /// What the kind's option is called in the menu, when the set says (the first set of a kind that does names it).
    pub kind_label: Option<String>,
    /// The value of the kind's option that chooses this set.
    pub value: String,
    pub value_label: String,
    /// The set has no option: it applies whenever its mod is loaded.
    pub always: bool,
    /// Who wins a conflict: the higher number; on a tie the mod that loads later.
    pub priority: i32,
    pub limits: FitLimits,
    /// The set's geometry file (the models), a path of the install's layout.
    pub geometry: Option<String>,
    /// The set's texture pack, a path of the install's layout. Its textures are added to the world once, except the ones a
    /// `[[texture]]` entry takes its picture from.
    pub textures: Option<String>,
    pub models: Vec<ModelEntry>,
    pub texture_entries: Vec<TextureEntry>,
    /// Objects the set places in the world, in file order.
    pub adds: Vec<Place>,
    /// The game the models and textures were made for, when the manifest names one (the game checks it has a reader).
    pub source_game: Option<String>,
}

impl ReplaceSet {
    /// The id of the option of this set's kind: `<kind>_variant`. `None` for a set with no kind.
    pub fn option(&self) -> Option<String> {
        (!self.kind.is_empty()).then(|| format!("{}_variant", self.kind))
    }

    /// How many models of the game the set replaces.
    pub fn model_targets(&self) -> usize {
        self.models.iter().map(|entry| entry.targets.len()).sum()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    set: RawSet,
    #[serde(default)]
    files: RawFiles,
    #[serde(default)]
    fit: RawLimits,
    #[serde(default)]
    model: Vec<RawModel>,
    #[serde(default)]
    texture: Vec<RawTexture>,
    #[serde(default)]
    add: Vec<RawPlace>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSet {
    kind: Option<String>,
    kind_label: Option<String>,
    value: Option<String>,
    value_label: Option<String>,
    #[serde(default)]
    always: bool,
    #[serde(default)]
    priority: i32,
    #[serde(default)]
    fit: SwapFit,
    source_game: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawFiles {
    geometry: Option<String>,
    textures: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModel {
    target: Option<String>,
    targets: Option<Vec<String>>,
    target_hash: Option<u32>,
    target_hashes: Option<Vec<u32>>,
    model: String,
    fit: Option<SwapFit>,
}

/// A kind id: `a-z`, `0-9` and `_`, 1 to 32 bytes, starting with a letter.
fn is_kind(text: &str) -> bool {
    text.len() <= MAX_ID_BYTES
        && text.bytes().next().is_some_and(|first| first.is_ascii_lowercase())
        && text.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// A value id: `a-z`, `0-9`, `_` and `-`, 1 to 32 bytes.
fn is_value(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_ID_BYTES
        && text.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-'))
}

fn is_label(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_LABEL_BYTES && !text.chars().any(char::is_control)
}

fn is_target_name(text: &str) -> bool {
    is_solid_name(text)
}

/// The targets one `[[model]]` lists: exactly one of `target`, `targets`, `target_hash`, `target_hashes`.
fn targets(model: &RawModel) -> Result<Vec<Target>, ContentError> {
    let mut out = Vec::new();
    let given =
        [model.target.is_some(), model.targets.is_some(), model.target_hash.is_some(), model.target_hashes.is_some()];
    if given.iter().filter(|given| **given).count() != 1 {
        return Err(field("model", "give `target`, `targets`, `target_hash` or `target_hashes`: exactly one"));
    }
    let names = model.target.iter().chain(model.targets.iter().flatten());
    for name in names {
        if !is_target_name(name) {
            return Err(ContentError::InvalidModel(name.clone()));
        }
        out.push(Target::Name(name.clone()));
    }
    out.extend(model.target_hash.iter().chain(model.target_hashes.iter().flatten()).map(|hash| Target::Hash(*hash)));
    if out.is_empty() {
        return Err(field("model", "a [[model]] lists at least one target"));
    }
    Ok(out)
}

/// Parse and check `replace/<stem>.toml`. `stem` is the file name without its extension and becomes the set's id.
pub fn parse(stem: &str, text: &str) -> Result<ReplaceSet, ContentError> {
    let Raw { format, set, files, fit, model, texture, add } =
        toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(format)?;
    if !is_value(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    let limits = fit.check()?;
    let head = head(stem, &set)?;
    for path in [&files.geometry, &files.textures].into_iter().flatten() {
        if !is_pack_path(path) {
            return Err(ContentError::InvalidText(path.clone()));
        }
    }
    let (models, texture_entries, adds) = entries(model, texture, add, files.textures.is_some(), head.fit)?;
    if models.is_empty() && texture_entries.is_empty() && adds.is_empty() {
        return Err(field("set", "a set replaces or adds at least one model or texture"));
    }
    if (!models.is_empty() || !adds.is_empty()) && files.geometry.is_none() {
        return Err(field("files.geometry", "needed: the set has models or objects"));
    }
    Ok(ReplaceSet {
        id: stem.to_owned(),
        kind: head.kind,
        kind_label: head.kind_label,
        value: head.value,
        value_label: head.value_label,
        always: set.always,
        priority: head.priority,
        limits,
        geometry: files.geometry,
        textures: files.textures,
        models,
        texture_entries,
        adds,
        source_game: head.source_game,
    })
}

/// The checked `[set]` table.
struct Head {
    kind: String,
    kind_label: Option<String>,
    value: String,
    value_label: String,
    priority: i32,
    fit: SwapFit,
    source_game: Option<String>,
}

fn head(stem: &str, set: &RawSet) -> Result<Head, ContentError> {
    let kind = set.kind.clone().unwrap_or_default();
    if !set.always && kind.is_empty() {
        return Err(field("set.kind", "needed unless the set is `always`"));
    }
    if !kind.is_empty() && !is_kind(&kind) {
        return Err(ContentError::InvalidId(kind));
    }
    let value = set.value.clone().unwrap_or_else(|| stem.to_owned());
    if !is_value(&value) {
        return Err(ContentError::InvalidId(value));
    }
    let value_label = set.value_label.clone().unwrap_or_else(|| value.clone());
    for label in [Some(&value_label), set.kind_label.as_ref()].into_iter().flatten() {
        if !is_label(label) {
            return Err(ContentError::InvalidName(label.clone()));
        }
    }
    if !(-MAX_PRIORITY..=MAX_PRIORITY).contains(&set.priority) {
        return Err(field("set.priority", "-1000 to 1000"));
    }
    let source_game = match &set.source_game {
        Some(name) if !is_value(name) => return Err(ContentError::InvalidId(name.clone())),
        other => other.clone(),
    };
    Ok(Head {
        kind,
        kind_label: set.kind_label.clone(),
        value,
        value_label,
        priority: set.priority,
        fit: set.fit,
        source_game,
    })
}

type Entries = (Vec<ModelEntry>, Vec<TextureEntry>, Vec<Place>);

/// The checked `[[model]]`, `[[texture]]` and `[[add]]` entries.
fn entries(
    raw_models: Vec<RawModel>,
    raw_textures: Vec<RawTexture>,
    raw_adds: Vec<RawPlace>,
    has_pack: bool,
    default_fit: SwapFit,
) -> Result<Entries, ContentError> {
    let mut seen = HashSet::new();
    let mut models = Vec::with_capacity(raw_models.len());
    for model in &raw_models {
        if !is_solid_name(&model.model) {
            return Err(ContentError::InvalidModel(model.model.clone()));
        }
        let targets = targets(model)?;
        for target in &targets {
            if !seen.insert(target.clone()) {
                return Err(field("model.target", "a target is replaced once in a set"));
            }
        }
        models.push(ModelEntry { targets, model: model.model.clone(), fit: model.fit.unwrap_or(default_fit) });
    }
    if models.iter().map(|entry| entry.targets.len()).sum::<usize>() > MAX_ENTRIES {
        return Err(field("model", "a set replaces at most 4096 models"));
    }
    if raw_textures.len() > MAX_ENTRIES {
        return Err(field("texture", "a set has at most 4096 textures"));
    }
    if raw_adds.len() > MAX_ADDS {
        return Err(field("add", "a set adds at most 65536 objects"));
    }
    let mut names = HashSet::new();
    let mut texture_entries = Vec::with_capacity(raw_textures.len());
    for texture in raw_textures {
        let entry = texture.check(has_pack)?;
        if !names.insert((entry.replaces.is_some(), entry.name.to_ascii_uppercase())) {
            return Err(field("texture", "a texture is replaced or added once in a set"));
        }
        texture_entries.push(entry);
    }
    let mut adds = Vec::with_capacity(raw_adds.len());
    for (at, raw_place) in raw_adds.into_iter().enumerate() {
        adds.push(place(at + 1, raw_place)?);
    }
    Ok((models, texture_entries, adds))
}
