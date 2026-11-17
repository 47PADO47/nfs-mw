//! The HD texture pack folder: loose DDS files that replace the world's textures (`docs/texture-packs.md`). Read once
//! at start-up; a new folder needs a restart. Unset, no textures are replaced.
//!
//! Config: `texture_packs` (a top-level key, or the same key under `[hd_content]`). Environment: `NFSMW_TEXTURE_PACKS`.
//! Command line: `--texture-packs <DIR>`.

use std::path::Path;
use std::sync::Once;

use clap::Args;
use toml::{Table, Value};

use super::super::Settings;
use super::super::net::InlineText;

/// The folder's path; the settings are `Copy`, so it is kept inline.
pub type TexturePackPath = InlineText<255>;

pub const ENV: &str = "NFSMW_TEXTURE_PACKS";
pub const KEY: &str = "texture_packs";

/// The folder of the config file: the top-level key, else the key of the `[hd_content]` table (a key written below a
/// table in TOML lands in that table). A value that is not text, or is too long, is reported and left unset.
pub fn read(table: &Table) -> Option<TexturePackPath> {
    let nested = table.get("hd_content").and_then(Value::as_table).and_then(|section| section.get(KEY));
    let value = table.get(KEY).or(nested)?;
    let Some(text) = value.as_str() else {
        log::warn!("config {KEY}: expected a folder path in quotes");
        return None;
    };
    parse(text).map_err(|e| log::warn!("config {KEY}: {e}")).ok()
}

/// The environment variable. A value that does not parse is ignored.
pub fn from_env(get: &impl Fn(&str) -> Option<String>) -> Option<TexturePackPath> {
    parse(&get(ENV)?).ok()
}

/// A folder path of at most 255 bytes. Empty text is a set value that names no folder, so it can switch a lower layer's
/// folder off.
pub fn parse(text: &str) -> Result<TexturePackPath, String> {
    TexturePackPath::new(text).map_err(|e| format!("the folder path must be {e}"))
}

/// The command-line option. Unset, the environment or the config file is in charge.
#[derive(Debug, Clone, Default, Args)]
pub struct TexturePacksArgs {
    /// Replace world textures with the DDS files of an HD texture pack folder; `""` for none
    /// [env NFSMW_TEXTURE_PACKS; config `texture_packs`; default none; read at start-up].
    #[arg(long, value_name = "DIR", value_parser = parse)]
    pub texture_packs: Option<TexturePackPath>,
}

/// Index the folder of `settings` for the world loader and log what it holds. The first call does it, before the world
/// loads; later ones (the settings are loaded more than once at start-up) change nothing, so a new folder needs a restart.
pub fn install(settings: &Settings) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| index(settings.hd_content.texture_packs.as_str()));
}

fn index(folder: &str) {
    if folder.is_empty() {
        nfsmw_data::texture_packs::set_root(None);
        return;
    }
    let root = Path::new(folder);
    if !root.is_dir() {
        log::warn!("texture packs: {} is not a folder; no textures are replaced", root.display());
    }
    let packs = nfsmw_data::texture_packs::set_root(Some(root));
    let count = packs.map_or(0, nfsmw_data::texture_packs::TexturePacks::file_count);
    // A warning, so release builds (which drop info lines) show it: the world will not look as the install has it.
    log::warn!("texture packs: {count} replacement texture file(s) under {}", root.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_top_level_key_wins_over_the_hd_content_table() {
        let top: Table = "texture_packs = 'C:\\Packs'\n[hd_content]\ntexture_packs = \"/other\"\n".parse().unwrap();
        assert_eq!(read(&top).unwrap().as_str(), "C:\\Packs");
        let nested: Table = "[hd_content]\ntexture_packs = \"/packs\"\n".parse().unwrap();
        assert_eq!(read(&nested).unwrap().as_str(), "/packs");
        let wrong: Table = "texture_packs = 4\n".parse().unwrap();
        assert_eq!(read(&wrong), None);
        assert_eq!(read(&Table::new()), None);
    }

    #[test]
    fn the_environment_and_long_paths() {
        let env = |name: &str| (name == ENV).then(|| " /packs ".to_owned());
        assert_eq!(from_env(&env).unwrap().as_str(), "/packs");
        assert!(parse(&"a".repeat(256)).is_err());
        assert!(parse("").unwrap().is_empty());
    }
}
