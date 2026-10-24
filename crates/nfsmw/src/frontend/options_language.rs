//! The Language row of the Gameplay options (`docs/specs/localization.md` section 6). Its choices are the languages on
//! offer (`i18n::choices`), each shown by its own name; picking one switches the game to it at once and records it for
//! the config file.

use std::sync::{Mutex, PoisonError};

use super::Data;
use super::steps::cycle;
use crate::i18n::{self, Choice};
use crate::settings::{LanguageTag, Partial, Settings};

/// The languages on offer and the language generation they were listed in. Listing them builds the catalogues, too
/// slow to repeat each time the row is drawn; they are listed again after a language change.
static CHOICES: Mutex<Option<(u64, Vec<Choice>)>> = Mutex::new(None);

fn choices() -> Vec<Choice> {
    let generation = i18n::generation();
    let mut cached = CHOICES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((listed, choices)) = cached.as_ref()
        && *listed == generation
    {
        return choices.clone();
    }
    let choices = i18n::choices();
    *cached = Some((generation, choices.clone()));
    choices
}

/// What the row shows: the current language's own name (its tag when it is not on offer).
pub(super) fn data() -> Data {
    let locale = i18n::current().locale.clone();
    let name = choices().into_iter().find(|choice| choice.locale == locale).map(|choice| choice.name);
    Data::Text(name.unwrap_or_else(|| locale.to_string()))
}

/// Moves to the next (`forward`) or previous language on offer, switches the game to it and records it in `changed`.
/// Returns whether the language changed: with one language on offer there is nothing to move to.
pub(super) fn step(s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
    let choices = choices();
    if choices.len() < 2 {
        return false;
    }
    let current = i18n::current().locale.clone();
    let at = choices.iter().position(|choice| choice.locale == current).unwrap_or(0);
    let next = &choices[cycle(at, choices.len(), forward)];
    if next.locale == current {
        return false;
    }
    let Some(tag) = LanguageTag::of(&next.locale) else {
        return false;
    };
    s.language = Some(tag);
    changed.language = Some(tag);
    i18n::switch(tag);
    true
}
