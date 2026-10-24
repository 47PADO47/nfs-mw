//! One language put together: its game layers (packs, then the install's tables) and its catalogue (packs, mods' own
//! messages, then the built-in messages), for each locale of its chain.

use std::path::PathBuf;

use blackbox_i18n::{Catalog, GameLayer, Locale, Source, Strings};
use game_install::GameDir;
use nfsmw_data::languages::{self, InstalledLanguage};

use super::{catalog, mods};

/// The language in use: its text and what else depends on it.
pub struct Active {
    pub locale: Locale,
    pub strings: Strings,
    /// The install's table that answers first (its fonts are the language's fonts). `None` without an install, or for a
    /// language only a catalogue has before English.
    pub table: Option<InstalledLanguage>,
    /// TrueType files of language packs for this language, first choice first, drawn for characters the game's fonts lack.
    pub fonts: Vec<PathBuf>,
}

impl Active {
    /// Whether the install speaks this language: the language's own table is a voice language. A language that only a
    /// catalogue or a pack provides is read, not heard.
    pub fn has_voice(&self) -> bool {
        let Some(game) = self.table.as_ref().and_then(|table| table.game) else { return false };
        game.voice && languages::by_tag(&self.locale.to_string()).is_some_and(|own| own.id == game.id)
    }

    /// Whether the language is English (subtitles show only for other languages).
    pub fn is_english(&self) -> bool {
        self.locale.language() == "en" && !self.locale.is_pseudo()
    }
}

/// The language `locale` from the install `dir` (with its mods). Without an install it is the built-in catalogue alone.
pub fn build(dir: Option<&GameDir>, locale: Locale) -> Active {
    let chain = locale.chain();
    let installed = dir.map(languages::installed).unwrap_or_default();
    let packs = mods::packs(&chain);

    // Within one locale the first layer that knows a label answers, so the packs go before the tables.
    let mut game: Vec<GameLayer> = packs.game;
    let mut table = None;
    for l in &chain {
        let Some(dir) = dir else { continue };
        let Some(found) = installed.iter().find(|t| Locale::parse(&t.tag).as_ref() == Some(l)) else { continue };
        match languages::table(dir, &found.file) {
            Ok(strings) => {
                game.push(GameLayer::table(l.clone(), &found.file, strings));
                table.get_or_insert_with(|| found.clone());
            }
            Err(e) => log::warn!("{}: {e:#}", found.file),
        }
    }

    let mut sources: Vec<Source> = packs.sources;
    sources.extend(catalog::built_in().into_iter().filter(|source| chain.contains(&source.locale)));
    let (catalog, problems) = Catalog::build(&sources, locale.is_pseudo());
    for problem in problems {
        log::warn!("translations: {problem}");
    }
    Active { strings: Strings::new(locale.clone(), game, catalog), locale, table, fonts: packs.fonts }
}
