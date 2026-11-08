//! `worlds/<id>.toml`: a second world a mod provides (docs/specs/second-world.md). The world is a set of map packs, one
//! scenery manifest each, in the folder `worlds/<id>/` of the mod; they load only while this world is the one played.
//!
//! This module checks the schema and the values. Whether the packs are valid is for the game to decide, with the same checks
//! as a `scenery/` pack.

use serde::Deserialize;

use super::{ContentError, is_id};

/// The folder inside a mod that holds one manifest per world, and, in a folder of the world's id, its packs.
pub const WORLDS_FOLDER: &str = "worlds";

/// The longest world name, in bytes.
const MAX_NAME_BYTES: usize = 64;

/// A checked `worlds/<id>.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldManifest {
    /// The world's id: the file name without `.toml`. It is what `--world` and the `world` setting name.
    pub id: String,
    /// What the Gameplay menu shows.
    pub name: String,
    /// Where the player starts, render space: x, y, z in metres.
    pub start: [f32; 3],
    /// The start heading in degrees, 0 along +x, turning towards +y.
    pub heading: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    world: RawWorld,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorld {
    name: String,
    start: RawStart,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStart {
    pos: [f32; 3],
    #[serde(default)]
    heading: f32,
}

/// Parse `worlds/<stem>.toml`. `stem` is the file name without its extension and becomes the world's id.
pub fn parse(stem: &str, text: &str) -> Result<WorldManifest, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    if !is_id(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    let name = raw.world.name;
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control) {
        return Err(ContentError::InvalidName(name));
    }
    let start = raw.world.start;
    if !start.pos.iter().all(|v| v.is_finite()) || !start.heading.is_finite() {
        return Err(ContentError::Field { field: "start", reason: "pos and heading are finite numbers" });
    }
    Ok(WorldManifest { id: stem.to_owned(), name, start: start.pos, heading: start.heading })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r#"
[world]
name = "Okayama"
start = { pos = [-228.0, -215.0, 26.5], heading = 95.0 }
"#;

    #[test]
    fn a_valid_world_parses() {
        let world = parse("okayama", TEXT).unwrap();
        assert_eq!(world.id, "okayama");
        assert_eq!(world.name, "Okayama");
        assert_eq!(world.start, [-228.0, -215.0, 26.5]);
        assert_eq!(world.heading, 95.0);
    }

    #[test]
    fn the_heading_is_optional() {
        let world = parse("w", &TEXT.replace(", heading = 95.0", "")).unwrap();
        assert_eq!(world.heading, 0.0);
    }

    #[test]
    fn a_bad_id_a_bad_name_and_an_unknown_key_are_refused() {
        assert_eq!(parse("not an id", TEXT), Err(ContentError::InvalidId("not an id".to_owned())));
        let nameless = TEXT.replace("Okayama", "");
        assert!(matches!(parse("okayama", &nameless), Err(ContentError::InvalidName(_))));
        let unknown = format!("{TEXT}\n[extra]\nx = 1\n");
        assert!(matches!(parse("okayama", &unknown), Err(ContentError::Parse(_))));
        let infinite = TEXT.replace("26.5", "inf");
        assert!(matches!(parse("okayama", &infinite), Err(ContentError::Field { .. })));
    }
}
