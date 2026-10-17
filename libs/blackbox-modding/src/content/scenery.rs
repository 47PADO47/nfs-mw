//! `scenery/<pack>.toml`: a map pack a mod adds (docs/specs/mod-world-content.md, section 2). The pack's model file is
//! `files/SCENERY/<PACK>.BUN` in the mod, and this manifest says where its objects go.
//!
//! This module checks the schema and the values. Whether the section number is free in the install's index, whether the
//! model file parses, and whether each model is in it, are for the game to decide.

use serde::Deserialize;

use super::{ContentError, is_id};

/// The largest rotation component, and the largest scale (a row's length). The game stores rotations as `i16 / 8192`,
/// so a component can hold up to about 4.0.
pub const MAX_COMPONENT: f32 = 4.0;

/// The longest model name, in bytes.
const MAX_MODEL_BYTES: usize = 63;

/// The largest streaming section number. The streaming index keeps numbers as `i16`.
const MAX_SECTION: u16 = i16::MAX as u16;

/// A checked `scenery/<pack>.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneryManifest {
    /// The pack key: the file name without `.toml`.
    pub key: String,
    /// The pack's file under the mod's `files/` folder, as written (`SCENERY/MONUMENT.BUN`).
    pub file: String,
    /// The streaming section number the pack takes. The game checks that the number is free.
    pub section: u16,
    /// The centre (x, y, z) and radius of the area the pack loads in, metres, render space.
    pub anchor: [f32; 4],
    /// The game the pack was made for, if the manifest names one. `None` means the running game.
    pub source_game: Option<String>,
    /// The objects to place, in file order. Never empty.
    pub places: Vec<Place>,
}

/// One object placed in the pack's area.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// The solid the object uses, by name. It must be in the pack's model file.
    pub model: String,
    /// The position, metres, render space (x forward, y left, z up).
    pub pos: [f32; 3],
    pub orientation: Orientation,
    /// The row length of the rotation. Between 0 and [`MAX_COMPONENT`].
    pub scale: f32,
    /// The low-detail model of the player view, if any.
    pub coarse: Option<String>,
    /// The model of the mirror and reflections, if not the same as `model`.
    pub mirror: Option<String>,
    /// The cull distance, metres. `None` keeps the game's own rule.
    pub draw_m: Option<f32>,
    /// The distance where the coarse model starts, metres. `None` keeps the game's own rule.
    pub lod_m: Option<f32>,
    pub collision: Collision,
    /// The raw exclude flags, if given. `None` keeps the game's default.
    pub exclude: Option<u32>,
    /// Raises the object's level-of-detail choice a little.
    pub boost: bool,
}

/// How a placed object is turned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    /// Degrees about the up axis.
    Yaw(f32),
    /// The rows of the rotation matrix, in the row-vector convention of the scenery format.
    Rows([[f32; 3]; 3]),
}

/// Whether a placed object has collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collision {
    None,
    /// Physics bounds keyed by the model's name. Not built yet: the game refuses it for now.
    Prop,
    /// The pack's own static collision. The game uses it when any placement asks for it.
    Static,
}

/// The file as serde reads it. Unknown keys are an error, so a key the game does not know refuses the pack.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    pack: RawPack,
    #[serde(default)]
    place: Vec<RawPlace>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPack {
    #[serde(default)]
    key: Option<String>,
    file: String,
    section: u16,
    anchor: [f32; 4],
    #[serde(default)]
    source_game: Option<String>,
    #[serde(default)]
    route: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlace {
    model: String,
    pos: [f32; 3],
    #[serde(default)]
    yaw: Option<f32>,
    #[serde(default)]
    rotation: Option<[[f32; 3]; 3]>,
    #[serde(default)]
    scale: Option<f32>,
    #[serde(default)]
    coarse: Option<String>,
    #[serde(default)]
    mirror: Option<String>,
    #[serde(default)]
    draw_m: Option<f32>,
    #[serde(default)]
    lod_m: Option<f32>,
    #[serde(default)]
    collision: Option<String>,
    #[serde(default)]
    exclude: Option<u32>,
    #[serde(default)]
    boost: bool,
}

/// Parse `scenery/<stem>.toml`. `stem` is the file name without its extension and becomes the pack key.
pub fn parse(stem: &str, text: &str) -> Result<SceneryManifest, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    validate(stem, raw)
}

fn validate(stem: &str, raw: Raw) -> Result<SceneryManifest, ContentError> {
    if !is_id(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    if raw.pack.key.as_deref().is_some_and(|key| key != stem) {
        let key = raw.pack.key.unwrap_or_default();
        return Err(ContentError::KeyMismatch { file: stem.to_owned(), key });
    }
    let file = pack_file(raw.pack.file)?;
    let section = section(raw.pack.section)?;
    let anchor = anchor(raw.pack.anchor)?;
    route(raw.pack.route)?;
    let source_game = source_game(raw.pack.source_game)?;
    if raw.place.is_empty() {
        return Err(ContentError::NoPlace);
    }
    let places = raw
        .place
        .into_iter()
        .enumerate()
        .map(|(at, place)| self::place(at + 1, place))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SceneryManifest { key: stem.to_owned(), file, section, anchor, source_game, places })
}

/// A pack file: a relative path under `files/`, ending in `.BUN`, with no `..`, no drive and no backslash.
fn pack_file(file: String) -> Result<String, ContentError> {
    let parts_ok = file.split('/').all(|part| !part.is_empty() && part != "..");
    let shape_ok = file.len() <= 256 && !file.contains([':', '\\']) && parts_ok;
    if !shape_ok || !file.to_ascii_lowercase().ends_with(".bun") {
        return Err(ContentError::InvalidPackFile(file));
    }
    Ok(file)
}

fn section(number: u16) -> Result<u16, ContentError> {
    if number == 0 || number > MAX_SECTION {
        return Err(ContentError::InvalidSection(number));
    }
    Ok(number)
}

fn anchor(anchor: [f32; 4]) -> Result<[f32; 4], ContentError> {
    let finite = anchor.iter().all(|v| v.is_finite());
    if !finite || anchor[3] <= 0.0 {
        return Err(ContentError::InvalidAnchor);
    }
    Ok(anchor)
}

/// Only the tile route exists in this delivery. Other routes are named in the spec and refused here.
fn route(route: Option<String>) -> Result<(), ContentError> {
    let Some(route) = route else { return Ok(()) };
    if route == "tile" {
        return Ok(());
    }
    Err(ContentError::RouteNotInV1(route))
}

fn source_game(name: Option<String>) -> Result<Option<String>, ContentError> {
    let Some(name) = name else { return Ok(None) };
    if !is_id(&name) {
        return Err(ContentError::InvalidId(name));
    }
    Ok(Some(name))
}

/// One placement, checked. `at` is its number in the file, from 1, for the message.
fn place(at: usize, raw: RawPlace) -> Result<Place, ContentError> {
    let fail = |reason: &'static str| ContentError::Place { at, reason };
    if !is_model(&raw.model) {
        return Err(ContentError::InvalidModel(raw.model));
    }
    if !raw.pos.iter().all(|v| v.is_finite()) {
        return Err(fail("pos is not finite"));
    }
    let orientation = orientation(raw.yaw, raw.rotation).map_err(fail)?;
    let scale = raw.scale.unwrap_or(1.0);
    if !(scale > 0.0 && scale <= MAX_COMPONENT) {
        return Err(fail("scale is above 0 and at most 4.0"));
    }
    let coarse = optional_model(raw.coarse).map_err(fail)?;
    let mirror = optional_model(raw.mirror).map_err(fail)?;
    let draw_m = distance(raw.draw_m).map_err(fail)?;
    let lod_m = distance(raw.lod_m).map_err(fail)?;
    if lod_m.zip(draw_m).is_some_and(|(lod, draw)| lod > draw) {
        return Err(fail("lod_m is beyond draw_m"));
    }
    let collision = collision(raw.collision).map_err(fail)?;
    Ok(Place {
        model: raw.model,
        pos: raw.pos,
        orientation,
        scale,
        coarse,
        mirror,
        draw_m,
        lod_m,
        collision,
        exclude: raw.exclude,
        boost: raw.boost,
    })
}

/// A yaw or a matrix, not both. Neither means no turn.
fn orientation(yaw: Option<f32>, rows: Option<[[f32; 3]; 3]>) -> Result<Orientation, &'static str> {
    match (yaw, rows) {
        (Some(_), Some(_)) => Err("yaw and rotation are both given"),
        (Some(yaw), None) if yaw.is_finite() => Ok(Orientation::Yaw(yaw)),
        (Some(_), None) => Err("yaw is not finite"),
        (None, Some(rows)) => rotation(rows),
        (None, None) => Ok(Orientation::Yaw(0.0)),
    }
}

fn rotation(rows: [[f32; 3]; 3]) -> Result<Orientation, &'static str> {
    let fits = rows.iter().flatten().all(|v| v.is_finite() && v.abs() <= MAX_COMPONENT);
    if !fits {
        return Err("a rotation component is not finite or is beyond 4.0");
    }
    Ok(Orientation::Rows(rows))
}

fn optional_model(name: Option<String>) -> Result<Option<String>, &'static str> {
    let Some(name) = name else { return Ok(None) };
    if !is_model(&name) {
        return Err("coarse or mirror model is not a valid name");
    }
    Ok(Some(name))
}

fn distance(metres: Option<f32>) -> Result<Option<f32>, &'static str> {
    match metres {
        None => Ok(None),
        Some(d) if d > 0.0 && d.is_finite() => Ok(Some(d)),
        Some(_) => Err("a distance must be positive"),
    }
}

fn collision(value: Option<String>) -> Result<Collision, &'static str> {
    match value.as_deref() {
        None | Some("none") => Ok(Collision::None),
        Some("prop") => Ok(Collision::Prop),
        Some("static") => Ok(Collision::Static),
        Some(_) => Err("collision must be none, prop or static"),
    }
}

/// A model name: letters, digits, `_` and `-`, 1 to 63 bytes.
fn is_model(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_MODEL_BYTES
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
