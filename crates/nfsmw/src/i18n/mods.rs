//! The translations mods ship: `lang/<locale>/` in a mod's folder (`docs/modding/translations.md`).
//!
//! ```text
//! lang/<locale>/*.ftl              the mod's own messages (namespace mod:<id>)
//! lang/<locale>/_core/*.ftl        the rewrite's messages, translated
//! lang/<locale>/_game/*.ftl        game labels by name, plain text with printf places
//! lang/<locale>/_mods/<id>/*.ftl   another mod's messages, translated
//! lang/<locale>/font.ttf | .otf    the fallback font for the characters the game's fonts lack
//! ```

use std::path::{Path, PathBuf};

use blackbox_i18n::{GameLayer, Locale, Source};

use super::{CORE, mod_namespace};
use crate::modding::content::{ModDir, mod_dirs};

/// The folder of a mod's translations.
pub const FOLDER: &str = "lang";

/// What the loaded mods give a chain of locales, highest priority first: a mod loaded later wins over an earlier one,
/// and a translation of someone else's messages wins over the owner's.
#[derive(Default)]
pub struct Packs {
    /// Game-label layers.
    pub game: Vec<GameLayer>,
    /// Catalogue sources: translations of others first, then each mod's own messages.
    pub sources: Vec<Source>,
    /// Fallback fonts.
    pub fonts: Vec<PathBuf>,
}

/// The packs of the loaded mods for the locales of `chain`.
pub fn packs(chain: &[Locale]) -> Packs {
    let mut packs = Packs::default();
    let mut own = Vec::new();
    for m in mod_dirs().iter().rev() {
        for (locale, folder) in locale_folders(m).into_iter().filter(|(l, _)| chain.contains(l)) {
            let source = |namespace: String, files| Source { namespace, locale: locale.clone(), origin: m.id.clone(), files };
            own.push(source(mod_namespace(&m.id), ftl_files(&folder)));
            packs.sources.push(source(CORE.to_owned(), ftl_files(&folder.join("_core"))));
            for (other, other_folder) in subfolders(&folder.join("_mods")) {
                packs.sources.push(source(mod_namespace(&other), ftl_files(&other_folder)));
            }
            let game = ftl_files(&folder.join("_game"));
            if !game.is_empty() {
                let (layer, problems) = GameLayer::pack(locale.clone(), &m.id, &game);
                problems.iter().for_each(|p| log::warn!("translations: {p}"));
                packs.game.push(layer);
            }
            packs.fonts.extend(["font.ttf", "font.otf"].iter().map(|name| folder.join(name)).filter(|p| p.is_file()));
        }
    }
    packs.sources.extend(own);
    packs.sources.retain(|source| !source.files.is_empty());
    packs
}

/// The `_core` sources of every locale the loaded mods translate the rewrite into (for the language list).
pub fn core_sources() -> Vec<Source> {
    let mut out = Vec::new();
    for m in mod_dirs().iter().rev() {
        for (locale, folder) in locale_folders(m) {
            let files = ftl_files(&folder.join("_core"));
            if files.is_empty() {
                continue;
            }
            out.push(Source { namespace: CORE.to_owned(), locale, origin: m.id.clone(), files });
        }
    }
    out
}

/// The locale folders of a mod's `lang/`: each subfolder whose name is a tag.
fn locale_folders(m: &ModDir) -> Vec<(Locale, PathBuf)> {
    subfolders(&m.dir.join(FOLDER))
        .into_iter()
        .filter_map(|(name, path)| {
            let Some(locale) = Locale::parse(&name) else {
                log::warn!("mod {}: {FOLDER}/{name} is not a language tag", m.id);
                return None;
            };
            Some((locale, path))
        })
        .collect()
}

/// The subfolders of `folder` by name, sorted. A missing folder has none.
fn subfolders(folder: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut out: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| Some((path.file_name()?.to_str()?.to_owned(), path)))
        .collect();
    out.sort();
    out
}

/// `(name, text)` of the `.ftl` files directly in `folder`, sorted by name. A file that is not UTF-8 is skipped.
fn ftl_files(folder: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ftl")))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            match std::fs::read_to_string(&path) {
                Ok(text) => Some((name, text)),
                Err(e) => {
                    log::warn!("{}: {e}", path.display());
                    None
                }
            }
        })
        .collect()
}
