//! The rewrite's own catalogue, built into the executable: `crates/nfsmw/i18n/<locale>/<domain>.ftl`. It holds only text
//! the rewrite wrote itself; the game's labels and their translations never go here (`docs/licensing.md`).

use blackbox_i18n::{Locale, Source};

use super::CORE;

/// `(locale, file, text)` of every built-in file. A new language is a new folder listed here.
const FILES: &[(&str, &str, &str)] = &[
    ("en", "career.ftl", include_str!("../../i18n/en/career.ftl")),
    ("en", "coop.ftl", include_str!("../../i18n/en/coop.ftl")),
    ("en", "dialogs.ftl", include_str!("../../i18n/en/dialogs.ftl")),
    ("en", "formats.ftl", include_str!("../../i18n/en/formats.ftl")),
    ("en", "hud.ftl", include_str!("../../i18n/en/hud.ftl")),
    ("en", "keys.ftl", include_str!("../../i18n/en/keys.ftl")),
    ("en", "lan.ftl", include_str!("../../i18n/en/lan.ftl")),
    ("en", "lan-menus.ftl", include_str!("../../i18n/en/lan-menus.ftl")),
    ("en", "languages.ftl", include_str!("../../i18n/en/languages.ftl")),
    ("en", "map.ftl", include_str!("../../i18n/en/map.ftl")),
    ("en", "menus.ftl", include_str!("../../i18n/en/menus.ftl")),
    ("en", "options.ftl", include_str!("../../i18n/en/options.ftl")),
    ("en", "options-graphics.ftl", include_str!("../../i18n/en/options-graphics.ftl")),
    ("en", "options-hud.ftl", include_str!("../../i18n/en/options-hud.ftl")),
    ("en", "prompts.ftl", include_str!("../../i18n/en/prompts.ftl")),
    ("en", "race.ftl", include_str!("../../i18n/en/race.ftl")),
    ("en", "results.ftl", include_str!("../../i18n/en/results.ftl")),
];

/// One source per built-in locale.
pub fn built_in() -> Vec<Source> {
    let mut out: Vec<Source> = Vec::new();
    for (tag, file, text) in FILES {
        let Some(locale) = Locale::parse(tag) else { continue };
        if let Some(source) = out.iter_mut().find(|source| source.locale == locale) {
            source.files.push(((*file).to_owned(), (*text).to_owned()));
            continue;
        }
        out.push(Source {
            namespace: CORE.to_owned(),
            locale,
            origin: "built-in".to_owned(),
            files: vec![((*file).to_owned(), (*text).to_owned())],
        });
    }
    out
}
