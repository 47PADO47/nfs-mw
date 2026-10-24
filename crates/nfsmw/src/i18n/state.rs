//! The process-wide current language and its change counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use blackbox_i18n::Locale;
use game_install::GameDir;

use super::build::{Active, build};
use super::select;
use crate::settings::LanguageTag;

struct State {
    /// The install the languages come from; `None` before [`start`].
    dir: Option<GameDir>,
    active: Arc<Active>,
}

static STATE: OnceLock<RwLock<State>> = OnceLock::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn state() -> &'static RwLock<State> {
    STATE.get_or_init(|| RwLock::new(State { dir: None, active: Arc::new(build(None, Locale::english())) }))
}

/// The language in use now. Cheap: an `Arc` clone.
pub fn current() -> Arc<Active> {
    Arc::clone(&state().read().unwrap_or_else(PoisonError::into_inner).active)
}

/// Counts the language changes. Text laid out for one generation is stale in the next.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

/// Chooses the language for the install `dir` (with its mods registered) and the player's setting, and makes it current.
pub fn start(dir: &GameDir, setting: Option<LanguageTag>) {
    let locale = select::choose(dir, setting);
    let active = build(Some(dir), locale);
    log::info!("language: {} ({})", active.locale, active.table.as_ref().map_or("no game table", |t| t.file.as_str()));
    install(Some(dir.clone()), active);
}

/// Makes `tag` the current language, read from the install given to [`start`]. Returns the locale now in use.
pub fn switch(tag: LanguageTag) -> Locale {
    let dir = state().read().unwrap_or_else(PoisonError::into_inner).dir.clone();
    let active = build(dir.as_ref(), tag.locale());
    let locale = active.locale.clone();
    log::info!("language switched to {locale}");
    install(dir, active);
    locale
}

/// The languages the player can pick, from the install given to [`start`].
pub fn choices() -> Vec<select::Choice> {
    let dir = state().read().unwrap_or_else(PoisonError::into_inner).dir.clone();
    select::available(dir.as_ref())
}

fn install(dir: Option<GameDir>, active: Active) {
    let mut guard = state().write().unwrap_or_else(PoisonError::into_inner);
    guard.dir = dir;
    guard.active = Arc::new(active);
    GENERATION.fetch_add(1, Ordering::AcqRel);
}
