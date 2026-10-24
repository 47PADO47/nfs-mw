//! Which language runs (`docs/specs/localization.md` section 3), and the languages the player can pick.

use blackbox_i18n::{Args, Catalog, Locale, negotiate, system_locales};
use game_install::GameDir;
use nfsmw_data::languages;

use super::{CORE, catalog, mods};
use crate::settings::LanguageTag;

/// The message that names a language in itself; a catalogue locale that has it can be picked.
const LANGUAGE_NAME: &str = "language-name";

/// A language the player can pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub locale: Locale,
    /// Its name in itself (`Deutsch`).
    pub name: String,
}

/// The language to run: the setting (command line, environment or config file), else the installer's registry
/// language, else the system's, matched against what is on offer; else English.
pub fn choose(dir: &GameDir, setting: Option<LanguageTag>) -> Locale {
    if let Some(tag) = setting {
        return tag.locale();
    }
    let offered: Vec<Locale> = available(Some(dir)).into_iter().map(|choice| choice.locale).collect();
    let registry = languages::registry_language().and_then(Locale::parse);
    for wanted in registry.into_iter().chain(system_locales()) {
        if let Some(found) = negotiate(&wanted, &offered) {
            return found;
        }
    }
    Locale::english()
}

/// The languages on offer, the install's tables first (English first), then those only a catalogue has. A table is named
/// by its catalogue's `language-name` when one exists, else by the game language's own name.
pub fn available(dir: Option<&GameDir>) -> Vec<Choice> {
    let mut sources = mods::core_sources();
    sources.extend(catalog::built_in());
    let (catalog, _) = Catalog::build(&sources, false);
    let name_in =
        |locale: &Locale| catalog.has(CORE, LANGUAGE_NAME, locale).then(|| own_name(&catalog, locale)).flatten();

    let mut out: Vec<Choice> = Vec::new();
    for table in dir.map(languages::installed).unwrap_or_default() {
        let Some(locale) = Locale::parse(&table.tag) else { continue };
        if out.iter().any(|choice| choice.locale == locale) {
            continue;
        }
        let name = name_in(&locale).unwrap_or(table.name);
        out.push(Choice { locale, name });
    }
    for locale in catalog.locales(CORE) {
        if out.iter().any(|choice| choice.locale == locale) {
            continue;
        }
        let Some(name) = name_in(&locale) else { continue };
        out.push(Choice { locale, name });
    }
    out
}

fn own_name(catalog: &Catalog, locale: &Locale) -> Option<String> {
    catalog.format(CORE, LANGUAGE_NAME, &Args::new(), std::slice::from_ref(locale)).map(|(name, _)| name)
}
