//! The texture replacement settings (`docs/texture-packs.md`): the HD texture pack folder, whose loose DDS files
//! replace game textures below every mod, and the size cap of every replacement file, mods' included. Both are read
//! once, when the install opens with its mods (`modding/textures.rs`); a change needs a restart. Nothing in the game
//! writes them.
//!
//! Config: `texture_packs` and `texture_max_size` (top-level keys, or the same keys under `[hd_content]`).
//! Environment: `NFSMW_TEXTURE_PACKS`, `NFSMW_TEXTURE_MAX_SIZE`. Command line: `--texture-packs <DIR>`,
//! `--texture-max-size <PIXELS>`.

use clap::Args;
use toml::{Table, Value};

use super::super::net::InlineText;

/// The folder's path; the settings are `Copy`, so it is kept inline.
pub type TexturePackPath = InlineText<255>;

pub const ENV: &str = "NFSMW_TEXTURE_PACKS";
pub const KEY: &str = "texture_packs";
pub const MAX_SIZE_ENV: &str = "NFSMW_TEXTURE_MAX_SIZE";
pub const MAX_SIZE_KEY: &str = "texture_max_size";

/// The smallest cap: below it a cap would blur even stock-sized textures.
pub const MIN_CAP: u32 = 256;
/// The largest side a DDS file may have (`blackbox_tpk::read_dds`).
pub const MAX_CAP: u32 = 16384;

/// The value of `key` in the config file: the top-level key, else the key of the `[hd_content]` table (a key written
/// below a table in TOML lands in that table).
fn config_value<'a>(table: &'a Table, key: &str) -> Option<&'a Value> {
    let nested = table.get("hd_content").and_then(Value::as_table).and_then(|section| section.get(key));
    table.get(key).or(nested)
}

/// The folder of the config file. A value that is not text, or is too long, is reported and left unset.
pub fn read(table: &Table) -> Option<TexturePackPath> {
    let value = config_value(table, KEY)?;
    let Some(text) = value.as_str() else {
        log::warn!("config {KEY}: expected a folder path in quotes");
        return None;
    };
    parse(text).map_err(|e| log::warn!("config {KEY}: {e}")).ok()
}

/// The size cap of the config file. A value that is not a whole number in range is reported and left unset.
pub fn read_max_size(table: &Table) -> Option<u32> {
    let value = config_value(table, MAX_SIZE_KEY)?;
    let Some(number) = value.as_integer() else {
        log::warn!("config {MAX_SIZE_KEY}: expected a whole number of pixels");
        return None;
    };
    parse_max_size(&number.to_string()).map_err(|e| log::warn!("config {MAX_SIZE_KEY}: {e}")).ok()
}

/// The environment variable. A value that does not parse is ignored.
pub fn from_env(get: &impl Fn(&str) -> Option<String>) -> Option<TexturePackPath> {
    parse(&get(ENV)?).ok()
}

/// The size cap of the environment. A value that does not parse is ignored.
pub fn max_size_from_env(get: &impl Fn(&str) -> Option<String>) -> Option<u32> {
    parse_max_size(get(MAX_SIZE_ENV)?.trim()).ok()
}

/// A folder path of at most 255 bytes. Empty text is a set value that names no folder, so it can switch a lower layer's
/// folder off.
pub fn parse(text: &str) -> Result<TexturePackPath, String> {
    TexturePackPath::new(text).map_err(|e| format!("the folder path must be {e}"))
}

/// A size cap: `0` for none, or a side length from [`MIN_CAP`] to [`MAX_CAP`] pixels.
pub fn parse_max_size(text: &str) -> Result<u32, String> {
    let value: u32 = text.parse().map_err(|_| format!("{text:?} is not a whole number of pixels"))?;
    if value != 0 && !(MIN_CAP..=MAX_CAP).contains(&value) {
        return Err(format!("{value} is out of range: 0 for no cap, or {MIN_CAP} to {MAX_CAP} pixels"));
    }
    Ok(value)
}

/// The command-line options. Unset, the environment or the config file is in charge.
#[derive(Debug, Clone, Default, Args)]
pub struct TexturePacksArgs {
    /// Replace game textures with the DDS files of an HD texture pack folder, below every mod; `""` for none
    /// [env NFSMW_TEXTURE_PACKS; config `texture_packs`; default none; read at start-up].
    #[arg(long, value_name = "DIR", value_parser = parse)]
    pub texture_packs: Option<TexturePackPath>,
    /// The largest side, in pixels, a replacement DDS file keeps: larger files drop their largest stored mip levels;
    /// 0 for no cap [env NFSMW_TEXTURE_MAX_SIZE; config `texture_max_size`; default 0; read at start-up].
    #[arg(long, value_name = "PIXELS", value_parser = parse_max_size)]
    pub texture_max_size: Option<u32>,
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

    #[test]
    fn the_size_cap_is_none_or_in_range() {
        assert_eq!(parse_max_size("0"), Ok(0));
        assert_eq!(parse_max_size("2048"), Ok(2048));
        assert!(parse_max_size("64").is_err());
        assert!(parse_max_size("32768").is_err());
        assert!(parse_max_size("big").is_err());
        let table: Table = "[hd_content]\ntexture_max_size = 4096\n".parse().unwrap();
        assert_eq!(read_max_size(&table), Some(4096));
        let env = |name: &str| (name == MAX_SIZE_ENV).then(|| " 1024 ".to_owned());
        assert_eq!(max_size_from_env(&env), Some(1024));
    }
}
