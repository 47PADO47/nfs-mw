//! The mod settings: `mods = off | allowlist | all` (ADR 0005 section 6) and the `mods_allowlist` list.
//!
//! `mods` is a layer like the others: command line > environment > config file > defaults. The allowlist is a list,
//! and [`super::Settings`] and [`super::Partial`] are `Copy`, so the list is read from the config file here instead.

use std::path::Path;
use std::str::FromStr;

use super::{Partial, Settings, env, file};

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
}

impl ModsSettings {
    /// Resolve the mod settings: `cli` over the environment over the config file over the defaults.
    pub fn load(cli: Partial) -> Self {
        let env = env::read(|name| std::env::var(name).ok());
        let config = Settings::config_path();
        let file = config.as_deref().map(file::read).unwrap_or_default();
        let mode = cli.or(env).or(file).mods.unwrap_or_default();
        let allowlist = config.as_deref().map(read_allowlist).unwrap_or_default();
        Self { mode, allowlist }
    }
}

/// The `mods_allowlist` key of the config file at `path`. A missing or broken file gives an empty list (the file
/// layer reports a broken file already).
pub fn read_allowlist(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_allowlist(&text)
}

/// The string items of `mods_allowlist` in `text`; anything else is skipped.
pub fn parse_allowlist(text: &str) -> Vec<String> {
    let Ok(table) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(items) = table.get("mods_allowlist").and_then(toml::Value::as_array) else {
        return Vec::new();
    };
    items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()
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
        assert_eq!(parse_allowlist(text), ["example.hello", "other.mod"]);
        assert!(parse_allowlist("mods_allowlist = \"one\"\n").is_empty());
        assert!(parse_allowlist("not = [toml").is_empty());
    }
}
