//! The one lookup every screen uses: game labels and catalogue messages over the locale chain.

use std::collections::HashSet;
use std::sync::Mutex;

use crate::{Args, Catalog, GameLayer, Locale, pseudo};

/// The text of one language: its chain, its game layers and its catalogue.
pub struct Strings {
    locale: Locale,
    chain: Vec<Locale>,
    game: Vec<GameLayer>,
    catalog: Catalog,
    /// Keys already reported missing, so each is logged once.
    missing: Mutex<HashSet<String>>,
}

impl Strings {
    /// `game` and the catalogue's sources are in priority order, highest first: within one locale the first layer that
    /// knows a key answers.
    pub fn new(locale: Locale, game: Vec<GameLayer>, catalog: Catalog) -> Self {
        let chain = locale.chain();
        Self { locale, chain, game, catalog, missing: Mutex::new(HashSet::new()) }
    }

    /// Nothing at all: every lookup misses (tests, and a run with no install).
    pub fn empty() -> Self {
        Self::new(Locale::english(), Vec::new(), Catalog::default())
    }

    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    pub fn chain(&self) -> &[Locale] {
        &self.chain
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// The text of game label `label` from the first locale of the chain that has it. A miss is logged once at debug level:
    /// screens look up labels their objects may not have.
    pub fn game(&self, label: u32) -> Option<String> {
        for locale in &self.chain {
            for layer in self.game.iter().filter(|layer| &layer.locale == locale) {
                if let Some(text) = layer.get(label) {
                    return Some(text);
                }
            }
        }
        if self.first_miss(&format!("label {label:#010x}")) {
            log::debug!("no text for game label {label:#010x} in {}", self.locale);
        }
        None
    }

    /// Whether the game's own table (not a pack) for `locale` is loaded.
    pub fn has_game_table(&self, locale: &Locale) -> bool {
        self.game.iter().any(|layer| &layer.locale == locale && layer.is_table())
    }

    /// Message `id` of `namespace`, or `None` when no locale of the chain has it.
    pub fn try_text(&self, namespace: &str, id: &str, args: &Args) -> Option<String> {
        let (text, answered) = self.catalog.format(namespace, id, args, &self.chain)?;
        if answered.is_pseudo() {
            return Some(pseudo::wrap(&text));
        }
        Some(text)
    }

    /// Message `id` of `namespace`; the id itself when no locale has it (logged once).
    pub fn text(&self, namespace: &str, id: &str, args: &Args) -> String {
        if let Some(text) = self.try_text(namespace, id, args) {
            return text;
        }
        if self.first_miss(&format!("{namespace} {id}")) {
            log::warn!("no message {id} in {namespace} for {}", self.locale);
        }
        id.to_owned()
    }

    fn first_miss(&self, key: &str) -> bool {
        let Ok(mut missing) = self.missing.lock() else { return false };
        missing.insert(key.to_owned())
    }
}
