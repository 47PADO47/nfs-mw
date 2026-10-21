//! The `[racers]` table of `mod.toml`: extra names for the computer-driven racers of a race. The game draws a racer's name
//! from its own list and from these.

use serde::Deserialize;

use super::ContentError;

/// Longest racer name, in characters: what the results screens have room for.
const MAX_NAME_CHARS: usize = 24;

/// A checked `[racers]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RacersSection {
    /// The names, as written. No name is empty, long or has a control character, and none is listed twice.
    pub names: Vec<String>,
}

/// The whole `mod.toml`, read for its `[racers]` table only.
#[derive(Deserialize)]
struct Root {
    #[serde(default)]
    racers: Option<Raw>,
}

/// The `[racers]` table as serde reads it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    names: Vec<String>,
}

/// Parse the text of a `mod.toml`. `Ok(None)` when it has no `[racers]` table.
pub fn parse(mod_toml: &str) -> Result<Option<RacersSection>, ContentError> {
    let root: Root = toml::from_str(mod_toml).map_err(|error| ContentError::Parse(error.to_string()))?;
    let Some(raw) = root.racers else { return Ok(None) };
    for (at, name) in raw.names.iter().enumerate() {
        let chars = name.chars().count();
        if name.trim().is_empty() || chars > MAX_NAME_CHARS || name.chars().any(char::is_control) {
            return Err(ContentError::InvalidRacerName(name.clone()));
        }
        if raw.names[..at].contains(name) {
            return Err(ContentError::RepeatedRacerName(name.clone()));
        }
    }
    Ok(Some(RacersSection { names: raw.names }))
}
