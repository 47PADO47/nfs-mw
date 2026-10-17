//! The mod settings: `mods = off | allowlist | all` (ADR 0005 section 6), the `mods_allowlist` list, and the
//! `mods_disabled` list of mods the player switched off in the mod manager.
//!
//! `mods` is a layer like the others: command line > environment > config file > defaults. The lists are lists, and
//! [`super::Settings`] and [`super::Partial`] are `Copy`, so the lists are read from the config file here instead.

use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::{Partial, Settings, env, file};

/// The config key of the allowlist (`mods = allowlist`).
pub const ALLOWLIST_KEY: &str = "mods_allowlist";
/// The config key of the mods switched off in the mod manager (`mods = all`).
pub const DISABLED_KEY: &str = "mods_disabled";

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

/// The string items of the list `key` in `text`; anything else is skipped.
pub fn parse_list(text: &str, key: &str) -> Vec<String> {
    let Ok(table) = text.parse::<Table>() else {
        return Vec::new();
    };
    let Some(items) = table.get(key).and_then(Value::as_array) else {
        return Vec::new();
    };
    items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()
}

/// The text of the file after the list `key` is set to `items` (empty text is an empty file). Every other key is left
/// as it is, as [`super::write`] does. A file that is not valid TOML is an error.
pub fn merge_list(existing: &str, key: &str, items: &[String]) -> Result<String> {
    let mut table: Table = existing.parse().context("the config file is not valid TOML")?;
    let array = items.iter().map(|item| Value::String(item.clone())).collect();
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
