//! `worlds/<id>.toml`: a world a mod provides (docs/specs/second-world.md, docs/specs/mod-content-replacement-worlds.md). A world
//! is a set of map packs, one scenery manifest each, in the folder `worlds/<id>/` of the mod; they load only while this world
//! is the one played. Another mod changes a world with a patch ([`super::world_patch`]).
//!
//! ```toml
//! [world]
//! name = "Okayama"
//! start = { pos = [-230.8, -214.7, 26.1], heading = 95.1 }   # render space, metres; degrees, 0 along +x
//! base = "rockport"                  # optional: an alternative map, the game's own city plus this world's packs
//! environment = { hour = 18.0, freeze = false, rain = 0.0 }  # optional: the clock and the weather when the world opens
//!
//! [[world.grid]]                     # optional, repeatable: start slots for a second player and for race grids (checked, not applied yet)
//! pos = [-233.8, -214.7, 26.1]
//! heading = 95.1
//! ```
//!
//! This module checks the schema and the values. Whether the packs are valid is for the game to decide, with the same checks as
//! a `scenery/` pack.

use std::fmt::Write as _;

use serde::Deserialize;

use super::{ContentError, is_id};

/// The folder inside a mod that holds one manifest per world, and, in a folder of the world's id, its packs.
pub const WORLDS_FOLDER: &str = "worlds";

/// The longest world name, in bytes.
const MAX_NAME_BYTES: usize = 64;
/// The most start slots a world lists.
pub const MAX_GRID_SLOTS: usize = 64;
/// The name of the city, which every alternative map is a version of, and which no mod provides as a world.
pub const CITY_BASE: &str = "rockport";

/// A place to stand: render-space position and heading in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub pos: [f32; 3],
    pub heading: f32,
}

/// What the world starts with: the clock and the weather.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Environment {
    /// The hour of the day the clock is set to (0 up to 24), or `None` to leave the game's own.
    pub hour: Option<f32>,
    /// Hold the clock at that hour.
    pub freeze: bool,
    /// How much it rains, 0 to 1 (and it rains all the time above 0), or `None` to leave the weather to the game.
    pub rain: Option<f32>,
}

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
    /// The world this one is a version of: `Some("rockport")` makes it an **alternative map** (the city plus this world's packs).
    pub base: Option<String>,
    pub environment: Option<Environment>,
    /// Extra start slots, after the first (which is `start`).
    pub grid: Vec<Slot>,
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
    base: Option<String>,
    environment: Option<RawEnvironment>,
    #[serde(default)]
    grid: Vec<RawStart>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub(super) struct RawStart {
    pub(super) pos: [f32; 3],
    #[serde(default)]
    pub(super) heading: f32,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub(super) struct RawEnvironment {
    hour: Option<f32>,
    #[serde(default)]
    freeze: bool,
    rain: Option<f32>,
}

impl RawStart {
    pub(super) fn check(self) -> Result<Slot, ContentError> {
        if !self.pos.iter().all(|v| v.is_finite()) || !self.heading.is_finite() {
            return Err(ContentError::Field { field: "start", reason: "pos and heading are finite numbers" });
        }
        Ok(Slot { pos: self.pos, heading: self.heading })
    }
}

impl RawEnvironment {
    pub(super) fn check(self) -> Result<Environment, ContentError> {
        let field = |reason| ContentError::Field { field: "environment", reason };
        if self.hour.is_some_and(|hour| !(0.0..24.0).contains(&hour)) {
            return Err(field("hour is from 0 up to 24"));
        }
        if self.rain.is_some_and(|rain| !(0.0..=1.0).contains(&rain)) {
            return Err(field("rain is from 0 to 1"));
        }
        if self.freeze && self.hour.is_none() {
            return Err(field("freeze needs an hour to hold the clock at"));
        }
        Ok(Environment { hour: self.hour, freeze: self.freeze, rain: self.rain })
    }
}

pub(super) fn check_name(name: &str) -> Result<(), ContentError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control) {
        return Err(ContentError::InvalidName(name.to_owned()));
    }
    Ok(())
}

/// Parse `worlds/<stem>.toml`. `stem` is the file name without its extension and becomes the world's id.
pub fn parse(stem: &str, text: &str) -> Result<WorldManifest, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    if !is_id(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    let RawWorld { name, start, base, environment, grid } = raw.world;
    check_name(&name)?;
    let start = start.check()?;
    if base.as_deref().is_some_and(|base| !base.eq_ignore_ascii_case(CITY_BASE)) {
        return Err(ContentError::Field { field: "base", reason: "only \"rockport\" (the city) can be a base" });
    }
    if grid.len() > MAX_GRID_SLOTS {
        return Err(ContentError::Field { field: "grid", reason: "at most 64 start slots" });
    }
    let grid = grid.into_iter().map(RawStart::check).collect::<Result<Vec<_>, _>>()?;
    let environment = environment.map(RawEnvironment::check).transpose()?;
    Ok(WorldManifest {
        id: stem.to_owned(),
        name,
        start: start.pos,
        heading: start.heading,
        base: base.map(|base| base.to_ascii_lowercase()),
        environment,
        grid,
    })
}

impl WorldManifest {
    /// A world with a name and a start, no base, no environment and no extra slots.
    pub fn new(id: &str, name: &str, start: [f32; 3], heading: f32) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            start,
            heading,
            base: None,
            environment: None,
            grid: Vec::new(),
        }
    }

    /// Whether this world is an alternative map: it plays in the city.
    pub fn is_alternative_map(&self) -> bool {
        self.base.is_some()
    }

    /// The text of `worlds/<id>.toml` for this manifest, which [`parse`] reads back to the same manifest. Mods that write a
    /// world (the converters) call this, so the schema has one writer.
    pub fn to_toml(&self) -> String {
        let [x, y, z] = self.start;
        let mut text = String::from("[world]\n");
        let _ = writeln!(text, "name = {:?}", self.name);
        let _ = writeln!(text, "start = {{ pos = [{x:.2}, {y:.2}, {z:.2}], heading = {:.1} }}", self.heading);
        if let Some(base) = &self.base {
            let _ = writeln!(text, "base = {base:?}");
        }
        if let Some(env) = &self.environment {
            let mut parts = Vec::new();
            if let Some(hour) = env.hour {
                parts.push(format!("hour = {hour:.2}"));
            }
            if env.freeze {
                parts.push("freeze = true".to_owned());
            }
            if let Some(rain) = env.rain {
                parts.push(format!("rain = {rain:.2}"));
            }
            let _ = writeln!(text, "environment = {{ {} }}", parts.join(", "));
        }
        for slot in &self.grid {
            let _ = writeln!(text, "\n[[world.grid]]");
            let _ = writeln!(text, "pos = [{:.2}, {:.2}, {:.2}]", slot.pos[0], slot.pos[1], slot.pos[2]);
            let _ = writeln!(text, "heading = {:.1}", slot.heading);
        }
        text
    }
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
        assert!(world.base.is_none() && world.environment.is_none() && world.grid.is_empty());
        assert!(!world.is_alternative_map());
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

    #[test]
    fn an_alternative_map_names_the_city_and_nothing_else_as_its_base() {
        let alt = TEXT.replace("[world]\n", "[world]\nbase = \"Rockport\"\n");
        let world = parse("alt", &alt).unwrap();
        assert_eq!(world.base.as_deref(), Some("rockport"));
        assert!(world.is_alternative_map());
        let other = TEXT.replace("[world]\n", "[world]\nbase = \"okayama\"\n");
        assert!(matches!(parse("alt", &other), Err(ContentError::Field { field: "base", .. })));
    }

    #[test]
    fn the_environment_and_the_grid_are_checked_and_written_back_the_same() {
        let mut world = WorldManifest::new("w", "A \"quoted\" name", [1.0, 2.0, 3.0], 90.0);
        world.base = Some("rockport".to_owned());
        world.environment = Some(Environment { hour: Some(22.5), freeze: true, rain: Some(0.25) });
        world.grid = vec![Slot { pos: [4.0, 5.0, 6.0], heading: 91.0 }, Slot { pos: [7.0, 8.0, 9.0], heading: 92.5 }];
        assert_eq!(parse("w", &world.to_toml()).unwrap(), world);
        for (what, text) in [
            ("an hour of 24", TEXT.replace("[world]\n", "[world]\nenvironment = { hour = 24.0 }\n")),
            ("rain above 1", TEXT.replace("[world]\n", "[world]\nenvironment = { rain = 1.5 }\n")),
            ("a freeze with no hour", TEXT.replace("[world]\n", "[world]\nenvironment = { freeze = true }\n")),
        ] {
            assert!(matches!(parse("w", &text), Err(ContentError::Field { field: "environment", .. })), "{what}");
        }
    }
}
