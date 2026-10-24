//! The game's languages: its language ids and files, the install's registry choice, and the language tables an install
//! (with its overlays) holds. Spec: `docs/specs/localization.md` sections 1 and 3.

use std::sync::Arc;

use anyhow::Result;
use blackbox_text::StringTable;
use game_install::{GameDir, RegistryValue};

use crate::shared_unwrapped;

/// The folder of the language tables.
pub const FOLDER: &str = "LANGUAGES";
/// English, the table every install has and the last of every chain.
pub const ENGLISH_FILE: &str = "LANGUAGES/English.bin";

/// One language the game knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameLanguage {
    /// The game's language id (`0` English to `13` Finnish).
    pub id: u32,
    /// The file stem under `LANGUAGES/`, as the game spells it.
    pub stem: &'static str,
    /// The BCP 47 tag.
    pub tag: &'static str,
    /// The language's name in itself, for the language list.
    pub endonym: &'static str,
    /// Whether the install speaks this language (phone calls are heard, not read).
    pub voice: bool,
}

const fn lang(id: u32, stem: &'static str, tag: &'static str, endonym: &'static str, voice: bool) -> GameLanguage {
    GameLanguage { id, stem, tag, endonym, voice }
}

/// The languages with a file on PC (ids 8 to 11 are console-only). **[decomp]**
pub const LANGUAGES: [GameLanguage; 10] = [
    lang(0, "English", "en", "English", true),
    lang(1, "French", "fr", "Français", true),
    lang(2, "German", "de", "Deutsch", true),
    lang(3, "Italian", "it", "Italiano", true),
    lang(4, "Spanish", "es", "Español", true),
    lang(5, "Dutch", "nl", "Nederlands", false),
    lang(6, "Swedish", "sv", "Svenska", false),
    lang(7, "Danish", "da", "Dansk", false),
    lang(12, "Polish", "pl", "Polski", false),
    lang(13, "Finnish", "fi", "Suomi", false),
];

/// Tables some installs and mods add that the game itself never loads: `(file stem, tag, name)`.
const OTHER_TABLES: [(&str, &str, &str); 4] =
    [("Russian", "ru", "Русский"), ("Czech", "cs", "Čeština"), ("Hungarian", "hu", "Magyar"), ("Portuguese", "pt", "Português")];

/// Files of the folder that are not languages.
const NOT_LANGUAGES: [&str; 3] = ["Labels", "Largest", "LanguageTextures"];

/// Where the installer writes the language it installed.
pub const REGISTRY_LANGUAGE: RegistryValue = RegistryValue { key: crate::game::REGISTRY_KEY, value: "Language" };

/// The registry's language names and their tags (spec section 1.2). **[decomp]**
const REGISTRY_NAMES: [(&str, &str); 16] = [
    ("English US", "en-US"),
    ("English UK", "en-GB"),
    ("French", "fr"),
    ("German", "de"),
    ("Italian", "it"),
    ("Spanish", "es"),
    ("Swedish", "sv"),
    ("Danish", "da"),
    ("Dutch", "nl"),
    ("Korean", "ko"),
    ("Chinese (Traditional)", "zh-Hant"),
    ("Chinese (Simplified)", "zh-Hans"),
    ("Japanese", "ja"),
    ("Thai", "th"),
    ("Polish", "pl"),
    ("Finnish", "fi"),
];

/// The tag of a registry language name, matched as the game does, ignoring case.
pub fn registry_tag(name: &str) -> Option<&'static str> {
    REGISTRY_NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(name.trim())).map(|(_, tag)| *tag)
}

/// The language the installer wrote to the registry, as a tag. `None` off Windows or when it is not there.
pub fn registry_language() -> Option<&'static str> {
    registry_tag(&game_install::registry_string(&REGISTRY_LANGUAGE)?)
}

/// The game language of a tag's language (`de-AT` is German).
pub fn by_tag(tag: &str) -> Option<&'static GameLanguage> {
    let language = tag.split(['-', '_']).next().unwrap_or(tag);
    LANGUAGES.iter().find(|l| l.tag.eq_ignore_ascii_case(language))
}

/// The game language of an id.
pub fn by_id(id: u32) -> Option<&'static GameLanguage> {
    LANGUAGES.iter().find(|l| l.id == id)
}

/// A language table the install (with its overlays) holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledLanguage {
    /// Game-relative path, `LANGUAGES/German.bin`.
    pub file: String,
    /// BCP 47 tag.
    pub tag: String,
    /// The name to list it under.
    pub name: String,
    /// The game's language, for a table the game itself knows.
    pub game: Option<&'static GameLanguage>,
}

/// The language tables in `LANGUAGES/`, English first, then by file name. A file counts when it holds a language chunk;
/// a stem no table knows gets its lower-case stem as its tag.
pub fn installed(dir: &GameDir) -> Vec<InstalledLanguage> {
    let mut out: Vec<InstalledLanguage> = dir
        .files_in(FOLDER)
        .into_iter()
        .filter_map(|name| {
            let (stem, ext) = name.rsplit_once('.')?;
            if !ext.eq_ignore_ascii_case("bin") || NOT_LANGUAGES.iter().any(|n| n.eq_ignore_ascii_case(stem)) {
                return None;
            }
            let file = format!("{FOLDER}/{name}");
            let bytes = shared_unwrapped(dir, &file).ok()?;
            if blackbox_chunk::find(&bytes, blackbox_text::LANGUAGE_CHUNK).is_none() {
                return None;
            }
            Some(describe(file, stem))
        })
        .collect();
    out.sort_by_key(|l| (l.game.map(|g| g.id) != Some(0), l.file.to_ascii_lowercase()));
    out
}

fn describe(file: String, stem: &str) -> InstalledLanguage {
    if let Some(game) = LANGUAGES.iter().find(|l| l.stem.eq_ignore_ascii_case(stem)) {
        return InstalledLanguage { file, tag: game.tag.to_owned(), name: game.endonym.to_owned(), game: Some(game) };
    }
    if let Some((_, tag, name)) = OTHER_TABLES.iter().find(|(s, _, _)| s.eq_ignore_ascii_case(stem)) {
        return InstalledLanguage { file, tag: (*tag).to_owned(), name: (*name).to_owned(), game: None };
    }
    InstalledLanguage { file, tag: stem.to_ascii_lowercase(), name: stem.to_owned(), game: None }
}

/// A language file's bytes without its compression wrapper, read once per session.
pub fn bytes(dir: &GameDir, file: &str) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, file)
}

/// A language file's string table.
pub fn table(dir: &GameDir, file: &str) -> Result<StringTable> {
    Ok(StringTable::from_file(&bytes(dir, file)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_match_ignoring_case() {
        assert_eq!(registry_tag("english uk"), Some("en-GB"));
        assert_eq!(registry_tag("German"), Some("de"));
        assert_eq!(registry_tag("Klingon"), None);
    }

    #[test]
    fn tags_find_their_game_language() {
        assert_eq!(by_tag("de-AT").map(|l| l.stem), Some("German"));
        assert_eq!(by_tag("en-GB").map(|l| l.id), Some(0));
        assert_eq!(by_tag("ru"), None);
        assert_eq!(by_id(12).map(|l| l.tag), Some("pl"));
    }

    #[test]
    fn stems_are_described() {
        assert_eq!(describe("LANGUAGES/GERMAN.BIN".into(), "GERMAN").tag, "de");
        assert_eq!(describe("LANGUAGES/Russian.bin".into(), "Russian").name, "Русский");
        assert_eq!(describe("LANGUAGES/Klingon.bin".into(), "Klingon").tag, "klingon");
    }
}
