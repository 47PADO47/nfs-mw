//! The `[custom_music]` section of the per-user config file: the player's own tracks on the radio
//! (`docs/specs/custom-music.md`, section 3).
//!
//! The section is read on its own, so the other settings never see it. A missing file or section is the defaults
//! (the feature is off). Each key is read on its own: a bad value is logged and only that key takes its default.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

/// The table's name in the config file.
const SECTION: &str = "custom_music";
/// The fade of a stopped custom track, in milliseconds (the original's `FadeoutTime`).
pub const DEFAULT_FADE_MS: u32 = 5000;
/// The longest fade the config may ask for.
pub const MAX_FADE_MS: u32 = 60_000;

/// How custom tracks and EA Trax songs share the radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomMode {
    /// Custom tracks take the place of the EA Trax songs in a context that has any.
    #[default]
    Replace,
    /// One EA Trax song, then one custom track, and so on.
    Mix,
}

/// The radio's contexts that custom sources feed: the front end (menus) and in game (free roam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomContext {
    Menu,
    FreeRoam,
}

/// Folders and playlist files, in the order the config lists them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    pub folders: Vec<PathBuf>,
    pub playlists: Vec<PathBuf>,
}

impl Sources {
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty() && self.playlists.is_empty()
    }
}

/// The `[custom_music]` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomMusic {
    /// Off by default: nothing is scanned or decoded.
    pub enabled: bool,
    pub mode: CustomMode,
    /// The starting order of the radio (`radio shuffle` changes it later).
    pub shuffle: bool,
    /// The fade-out of a custom track stopped before its end, in milliseconds (0 to [`MAX_FADE_MS`]).
    pub fade_ms: u32,
    /// Sources for every context.
    pub global: Sources,
    pub menu: Sources,
    pub free_roam: Sources,
    /// Read from the config and kept for the race context, which the radio does not play yet (spec section 2.2).
    #[allow(dead_code)]
    pub race: Sources,
    /// Read from the config and kept for the pursuit context, which the radio does not play yet (spec section 2.2).
    #[allow(dead_code)]
    pub pursuit: Sources,
}

impl Default for CustomMusic {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: CustomMode::default(),
            shuffle: false,
            fade_ms: DEFAULT_FADE_MS,
            global: Sources::default(),
            menu: Sources::default(),
            free_roam: Sources::default(),
            race: Sources::default(),
            pursuit: Sources::default(),
        }
    }
}

impl CustomMusic {
    /// The sources of `context`: its own when it names any, else the global ones.
    pub fn sources_for(&self, context: CustomContext) -> &Sources {
        let own = match context {
            CustomContext::Menu => &self.menu,
            CustomContext::FreeRoam => &self.free_roam,
        };
        if own.is_empty() { &self.global } else { own }
    }

    /// The config file's section, read from the per-user config file (the defaults when there is none).
    pub fn load() -> Self {
        let Some(path) = super::Settings::config_path() else { return Self::default() };
        read(&path)
    }
}

/// Read the section from the config file at `path`. A missing file is the defaults; relative paths in it are
/// relative to the file's folder.
pub fn read(path: &Path) -> CustomMusic {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text, path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => CustomMusic::default(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            CustomMusic::default()
        }
    }
}

/// Parse the whole config file's text. A file that is not TOML leaves the feature off.
pub fn parse(text: &str, path: &Path) -> CustomMusic {
    match text.parse::<Table>() {
        Ok(table) => from_table(&table, path.parent(), &path.display().to_string()),
        Err(e) => {
            log::warn!("ignoring [{SECTION}] in {}: {e}", path.display());
            CustomMusic::default()
        }
    }
}

/// Read the section out of a parsed config file. `base` is the folder that relative paths are relative to.
fn from_table(root: &Table, base: Option<&Path>, origin: &str) -> CustomMusic {
    let Some(value) = root.get(SECTION) else { return CustomMusic::default() };
    let Some(table) = value.as_table() else {
        log::warn!("{origin}: [{SECTION}] must be a table; using the defaults");
        return CustomMusic::default();
    };
    let defaults = CustomMusic::default();
    CustomMusic {
        enabled: boolean(table, "enabled", origin).unwrap_or(defaults.enabled),
        mode: mode(table, origin).unwrap_or(defaults.mode),
        shuffle: boolean(table, "shuffle", origin).unwrap_or(defaults.shuffle),
        fade_ms: fade_ms(table, origin).unwrap_or(defaults.fade_ms),
        global: sources(table, base, origin),
        menu: context_table(table, "menu", base, origin),
        free_roam: context_table(table, "free_roam", base, origin),
        race: context_table(table, "race", base, origin),
        pursuit: context_table(table, "pursuit", base, origin),
    }
}

fn boolean(table: &Table, key: &str, origin: &str) -> Option<bool> {
    let value = table.get(key)?;
    let parsed = value.as_bool();
    if parsed.is_none() {
        log::warn!("{origin}: [{SECTION}] {key} must be true or false; using the default");
    }
    parsed
}

fn mode(table: &Table, origin: &str) -> Option<CustomMode> {
    let value = table.get("mode")?;
    let Some(text) = value.as_str() else {
        log::warn!("{origin}: [{SECTION}] mode must be \"replace\" or \"mix\"; using the default");
        return None;
    };
    match text.trim().to_ascii_lowercase().as_str() {
        "replace" => Some(CustomMode::Replace),
        "mix" => Some(CustomMode::Mix),
        _ => {
            log::warn!("{origin}: [{SECTION}] mode must be \"replace\" or \"mix\", not {text:?}; using the default");
            None
        }
    }
}

/// The fade in milliseconds. A negative number is 0; a number above the longest fade is that fade.
fn fade_ms(table: &Table, origin: &str) -> Option<u32> {
    let value = table.get("fade_ms")?;
    let Value::Integer(ms) = value else {
        log::warn!("{origin}: [{SECTION}] fade_ms must be a whole number of milliseconds; using the default");
        return None;
    };
    Some(u32::try_from((*ms).clamp(0, i64::from(MAX_FADE_MS))).unwrap_or(MAX_FADE_MS))
}

/// The `[custom_music.<name>]` table's sources; a missing table has none.
fn context_table(table: &Table, name: &str, base: Option<&Path>, origin: &str) -> Sources {
    let Some(value) = table.get(name) else { return Sources::default() };
    let Some(inner) = value.as_table() else {
        log::warn!("{origin}: [{SECTION}.{name}] must be a table; ignored");
        return Sources::default();
    };
    sources(inner, base, origin)
}

/// The `folders` and `playlists` arrays of a table.
fn sources(table: &Table, base: Option<&Path>, origin: &str) -> Sources {
    Sources { folders: paths(table, "folders", base, origin), playlists: paths(table, "playlists", base, origin) }
}

/// An array of strings as paths. A non-string item is logged and skipped.
fn paths(table: &Table, key: &str, base: Option<&Path>, origin: &str) -> Vec<PathBuf> {
    let Some(value) = table.get(key) else { return Vec::new() };
    let Some(items) = value.as_array() else {
        log::warn!("{origin}: [{SECTION}] {key} must be a list of paths; ignored");
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let Some(text) = item.as_str() else {
                log::warn!("{origin}: [{SECTION}] {key} has an item that is not a path; skipped");
                return None;
            };
            Some(resolve(base, text))
        })
        .collect()
}

/// A path from the config file: relative ones are joined to the file's folder.
fn resolve(base: Option<&Path>, text: &str) -> PathBuf {
    let path = PathBuf::from(text);
    match base {
        Some(base) if path.is_relative() => base.join(path),
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_text(text: &str) -> CustomMusic {
        parse(text, Path::new("C:/cfg/nfsmw.toml"))
    }

    #[test]
    fn a_missing_section_is_the_defaults_and_the_feature_is_off() {
        let got = parse_text("radio = true\n");
        assert_eq!(got, CustomMusic::default());
        assert!(!got.enabled);
        assert_eq!(got.fade_ms, 5000);
        assert_eq!(got.mode, CustomMode::Replace);
    }

    #[test]
    fn every_key_is_read_and_relative_paths_follow_the_config_folder() {
        // Literal strings ('...') so Windows paths need no escaping.
        let absolute = std::env::temp_dir().join("music");
        let text = format!(
            "[custom_music]\n\
             enabled = true\n\
             mode = \"Mix\"\n\
             shuffle = true\n\
             fade_ms = 1500\n\
             folders = ['{}', 'rel/folder']\n\
             playlists = ['list.m3u8']\n\
             [custom_music.menu]\n\
             folders = ['menu-folder']\n",
            absolute.display()
        );
        let got = parse_text(&text);
        assert!(got.enabled && got.shuffle);
        assert_eq!(got.mode, CustomMode::Mix);
        assert_eq!(got.fade_ms, 1500);
        assert_eq!(got.global.folders, [absolute, Path::new("C:/cfg").join("rel/folder")]);
        assert_eq!(got.global.playlists, [Path::new("C:/cfg").join("list.m3u8")]);
        assert_eq!(got.menu.folders, [Path::new("C:/cfg").join("menu-folder")]);
    }

    #[test]
    fn a_bad_value_takes_its_own_default_and_the_others_stay() {
        let text = r#"
            [custom_music]
            enabled = "yes"
            mode = "loud"
            shuffle = true
            fade_ms = "soon"
            folders = [1, "ok"]
        "#;
        let got = parse_text(text);
        assert!(!got.enabled);
        assert_eq!(got.mode, CustomMode::Replace);
        assert!(got.shuffle);
        assert_eq!(got.fade_ms, DEFAULT_FADE_MS);
        assert_eq!(got.global.folders, [Path::new("C:/cfg").join("ok")]);
    }

    #[test]
    fn the_fade_is_clamped_to_its_range() {
        let high = parse_text("[custom_music]\nfade_ms = 999999\n");
        assert_eq!(high.fade_ms, MAX_FADE_MS);
        let low = parse_text("[custom_music]\nfade_ms = -20\n");
        assert_eq!(low.fade_ms, 0);
    }

    #[test]
    fn a_file_that_is_not_toml_leaves_the_feature_off() {
        assert_eq!(parse_text("[custom_music\nenabled = true"), CustomMusic::default());
    }

    #[test]
    fn a_context_uses_its_own_sources_or_the_global_ones() {
        let got = parse_text("[custom_music]\nfolders = [\"g\"]\n[custom_music.race]\nfolders = [\"r\"]\n");
        let global = Path::new("C:/cfg").join("g");
        assert_eq!(got.sources_for(CustomContext::Menu).folders, std::slice::from_ref(&global));
        assert_eq!(got.sources_for(CustomContext::FreeRoam).folders, [global]);
        assert_eq!(got.race.folders, [Path::new("C:/cfg").join("r")]);
    }
}
