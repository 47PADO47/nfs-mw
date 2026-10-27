//! The mod settings: `mods = off | allowlist | all` (ADR 0005 section 6), the `mods_allowlist` list, and the
//! `mods_disabled` list of mods the player switched off in the mod manager.
//!
//! `mods` is a layer like the others: command line > environment > config file > defaults. The lists are lists, and
//! [`super::Settings`] and [`super::Partial`] are `Copy`, so the lists are read from the config file here instead.
//!
//! The `mods` key has two shapes (docs/modding/api-config.md):
//!
//! ```toml
//! mods = "all"                  # the legacy string: the mode only
//! mods_allowlist = ["a.mod"]    # the legacy lists
//!
//! [mods]                        # the table: the mode, the lists, and one table per mod
//! mode = "allowlist"
//! allowlist = ["a.mod"]
//! disabled = ["b.mod"]
//! [mods.a-mod]                  # the configuration of the mod `a-mod` (`bb.config`)
//! key = "value"
//! ```
//!
//! A key and a table cannot share a name in TOML, so a file has one shape or the other. The precedence rule for the lists
//! is: **the table's `allowlist` and `disabled` win over the legacy `mods_allowlist` and `mods_disabled`**; a legacy list is
//! used only when the table has no such key (or `mods` is the string). The mode has one source, `mods`, whichever shape.
//! Writers keep the shape they find: the mod manager writes `mods.allowlist` / `mods.disabled` into a `[mods]` table and
//! the legacy keys otherwise, and neither touches the `[mods.<id>]` tables.

use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::{Partial, Settings, env, file};

/// The config key of the allowlist (`mods = allowlist`).
pub const ALLOWLIST_KEY: &str = "mods_allowlist";
/// The config key of the mods switched off in the mod manager (`mods = all`).
pub const DISABLED_KEY: &str = "mods_disabled";
/// The key of the mod mode, and of the table that holds the mode, the lists and the mods' own tables.
pub const MODS_KEY: &str = "mods";
/// The mode inside the `[mods]` table.
const MODE_KEY: &str = "mode";
/// The allowlist inside the `[mods]` table (the same list as [`ALLOWLIST_KEY`]).
pub const TABLE_ALLOWLIST_KEY: &str = "allowlist";
/// The switched-off list inside the `[mods]` table (the same list as [`DISABLED_KEY`]).
pub const TABLE_DISABLED_KEY: &str = "disabled";

/// Which mods may load: `off` loads none, `allowlist` only the ids in `mods_allowlist`, `all` every mod in the folders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModsMode {
    Off,
    Allowlist,
    /// The default: ADR 0005 section 6 proposes loading every mod in the folder.
    #[default]
    All,
}

impl FromStr for ModsMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "allowlist" => Ok(Self::Allowlist),
            "all" => Ok(Self::All),
            _ => Err(format!("expected off, allowlist or all, got {s:?}")),
        }
    }
}

/// The resolved mod settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModsSettings {
    pub mode: ModsMode,
    /// The mod ids that may load in `allowlist` mode.
    pub allowlist: Vec<String>,
    /// The mod ids switched off in the mod manager. They do not load in `all` mode.
    pub disabled: Vec<String>,
}

impl ModsSettings {
    /// Resolve the mod settings: `cli` over the environment over the config file over the defaults.
    pub fn load(cli: Partial) -> Self {
        let env = env::read(|name| std::env::var(name).ok());
        let config = Settings::config_path();
        let file = config.as_deref().map(file::read).unwrap_or_default();
        let mode = cli.or(env).or(file).mods.unwrap_or_default();
        let (allowlist, disabled) = match config.as_deref() {
            Some(path) => (read_list(path, ALLOWLIST_KEY), read_list(path, DISABLED_KEY)),
            None => (Vec::new(), Vec::new()),
        };
        Self { mode, allowlist, disabled }
    }
}

/// The list under `key` in the config file at `path`. A missing or broken file gives an empty list (the file layer
/// reports a broken file already).
pub fn read_list(path: &Path, key: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_list(&text, key)
}

/// The key inside the `[mods]` table that holds the same list as the legacy `key`.
fn table_key(key: &str) -> Option<&'static str> {
    match key {
        ALLOWLIST_KEY => Some(TABLE_ALLOWLIST_KEY),
        DISABLED_KEY => Some(TABLE_DISABLED_KEY),
        _ => None,
    }
}

/// The string items of the list `key` (`mods_allowlist` or `mods_disabled`) in `text`; anything else is skipped. The list
/// in the `[mods]` table (`allowlist`, `disabled`) wins when the table has it; otherwise the legacy key is read.
pub fn parse_list(text: &str, key: &str) -> Vec<String> {
    let Ok(table) = text.parse::<Table>() else {
        return Vec::new();
    };
    let in_table = table_key(key).and_then(|inner| table.get(MODS_KEY)?.as_table()?.get(inner)?.as_array());
    let Some(items) = in_table.or_else(|| table.get(key).and_then(Value::as_array)) else {
        return Vec::new();
    };
    items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()
}

/// The mode `mods` sets in a parsed config file, in either shape. A bad value is reported and ignored; a `[mods]` table with
/// no `mode` sets none.
pub fn read_mode(table: &Table, origin: &str) -> Option<ModsMode> {
    let value = match table.get(MODS_KEY)? {
        Value::Table(inner) => inner.get(MODE_KEY)?,
        other => other,
    };
    let Some(text) = value.as_str() else {
        log::warn!("{origin}: ignoring the mods mode: expected a string");
        return None;
    };
    text.parse().map_err(|e| log::warn!("{origin}: ignoring the mods mode: {e}")).ok()
}

/// The configuration the config file text keeps for mod `id`: the table `[mods.<id>]`, as its own keys. `None` when `mods`
/// is not a table (the legacy string), the table has no such mod, or the text is not TOML. The id is the exact key, so a
/// mod id with dots is written quoted: `[mods."my.mod"]`.
pub fn mod_section(text: &str, id: &str) -> Option<Table> {
    let table: Table = text.parse().ok()?;
    section_in(&table, id).cloned()
}

/// [`mod_section`] over a config file that is already parsed.
pub fn section_in<'a>(table: &'a Table, id: &str) -> Option<&'a Table> {
    table.get(MODS_KEY)?.as_table()?.get(id)?.as_table()
}

/// The text of the file after the list `key` is set to `items` (empty text is an empty file). Every other key is left
/// as it is, as [`super::write`] does. A file that is not valid TOML is an error.
pub fn merge_list(existing: &str, key: &str, items: &[String]) -> Result<String> {
    let mut table: Table = existing.parse().context("the config file is not valid TOML")?;
    let array = items.iter().map(|item| Value::String(item.clone())).collect();
    // A `[mods]` table keeps its lists inside it, where they win over the legacy keys; the string shape keeps the legacy key.
    if let (Some(inner), Some(Value::Table(mods))) = (table_key(key), table.get_mut(MODS_KEY)) {
        mods.insert(inner.to_owned(), Value::Array(array));
        return toml::to_string(&table).context("serializing the config file");
    }
    table.insert(key.to_owned(), Value::Array(array));
    toml::to_string(&table).context("serializing the config file")
}

/// Sets the list `key` in the file at `path` to `items`, creating the file (and its folder) if needed.
pub fn write_list(path: &Path, key: &str, items: &[String]) -> Result<()> {
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let text = merge_list(&existing, key, items).with_context(|| path.display().to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_has_three_names() {
        assert_eq!("off".parse::<ModsMode>(), Ok(ModsMode::Off));
        assert_eq!(" Allowlist ".parse::<ModsMode>(), Ok(ModsMode::Allowlist));
        assert_eq!("ALL".parse::<ModsMode>(), Ok(ModsMode::All));
        assert!("some".parse::<ModsMode>().is_err());
    }

    #[test]
    fn the_default_mode_loads_every_mod() {
        assert_eq!(ModsMode::default(), ModsMode::All);
        assert_eq!(ModsSettings::default().mode, ModsMode::All);
    }

    #[test]
    fn the_command_line_layer_wins_over_the_file() {
        let cli = Partial { mods: Some(ModsMode::Off), ..Partial::default() };
        assert_eq!(cli.or(Partial::default()).mods, Some(ModsMode::Off));
    }

    #[test]
    fn reads_the_allowlist_and_skips_other_items() {
        let text = "mods_allowlist = [\"example.hello\", 7, \"other.mod\"]\n";
        assert_eq!(parse_list(text, ALLOWLIST_KEY), ["example.hello", "other.mod"]);
        assert!(parse_list("mods_allowlist = \"one\"\n", ALLOWLIST_KEY).is_empty());
        assert!(parse_list("not = [toml", ALLOWLIST_KEY).is_empty());
    }

    #[test]
    fn reads_the_disabled_list_under_its_own_key() {
        let text = "mods_allowlist = [\"a.mod\"]\nmods_disabled = [\"b.mod\"]\n";
        assert_eq!(parse_list(text, DISABLED_KEY), ["b.mod"]);
        assert_eq!(parse_list(text, ALLOWLIST_KEY), ["a.mod"]);
    }

    #[test]
    fn writing_a_list_keeps_the_other_keys() {
        let existing = "backend = \"vulkan\"\nmods = \"all\"\n";
        let text = merge_list(existing, DISABLED_KEY, &["x.mod".to_owned()]).unwrap();
        assert_eq!(parse_list(&text, DISABLED_KEY), ["x.mod"]);
        let table: Table = text.parse().unwrap();
        assert_eq!(table.get("backend").and_then(Value::as_str), Some("vulkan"));
        assert_eq!(table.get("mods").and_then(Value::as_str), Some("all"));
    }

    #[test]
    fn an_empty_list_is_written_and_a_broken_file_is_refused() {
        let text = merge_list("", DISABLED_KEY, &[]).unwrap();
        assert!(parse_list(&text, DISABLED_KEY).is_empty());
        assert!(text.contains("mods_disabled"), "the key stays so the file says what is off");
        assert!(merge_list("not = [toml", DISABLED_KEY, &[]).is_err());
    }
}
