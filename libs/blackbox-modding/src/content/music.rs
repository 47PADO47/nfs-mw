//! The `[music]` table of `mod.toml`: the contexts the mod's `music/` tracks join, and whether they play even when the
//! player's own custom music is off.

use serde::Deserialize;

use super::ContentError;

/// A checked `[music]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicSection {
    /// The contexts the tracks may play in, by name (`menu`, `free_roam`), as written. The game decides which names it
    /// knows. The list has no repeats.
    pub contexts: Vec<String>,
    /// Play the tracks even when the custom music setting is off.
    pub always: bool,
}

/// The whole `mod.toml`, read for its `[music]` table only. The other tables are not checked here.
#[derive(Deserialize)]
struct Root {
    #[serde(default)]
    music: Option<Raw>,
}

/// The `[music]` table as serde reads it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default = "default_contexts")]
    contexts: Vec<String>,
    #[serde(default)]
    always: bool,
}

/// A `[music]` table without `contexts` plays in every context the game has.
fn default_contexts() -> Vec<String> {
    vec!["menu".to_owned(), "free_roam".to_owned()]
}

/// Parse the text of a `mod.toml`. `Ok(None)` when it has no `[music]` table.
pub fn parse(mod_toml: &str) -> Result<Option<MusicSection>, ContentError> {
    let root: Root = toml::from_str(mod_toml).map_err(|error| ContentError::Parse(error.to_string()))?;
    let Some(raw) = root.music else { return Ok(None) };
    validate(raw).map(Some)
}

fn validate(raw: Raw) -> Result<MusicSection, ContentError> {
    if raw.contexts.is_empty() {
        return Err(ContentError::NoMusicContext);
    }
    for (at, context) in raw.contexts.iter().enumerate() {
        if !is_context_name(context) {
            return Err(ContentError::InvalidMusicContext(context.clone()));
        }
        if raw.contexts[..at].contains(context) {
            return Err(ContentError::RepeatedMusicContext(context.clone()));
        }
    }
    Ok(MusicSection { contexts: raw.contexts, always: raw.always })
}

/// A context name: lower-case letters and `_`, 1 to 32 bytes.
fn is_context_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 32 && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'_')
}
