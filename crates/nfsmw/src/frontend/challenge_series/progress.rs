//! What the player has won in the series, and so which events are open. The original keeps it in the race score flags
//! of the profile (`GRaceDatabase::IsQuickRaceComplete`, `IsQuickRaceUnlocked`); this build keeps the won event ids in
//! `challenge_series.toml` next to the config file, whatever profile is played. An event is open when its record says
//! it starts open, or when a won event lists it in `UnlockRaces` (`docs/specs/challenge-series.md` section 4).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;

use super::board::{Board, Entry};

const FILE: &str = "challenge_series.toml";

/// The won events, by event id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Progress {
    won: BTreeSet<String>,
}

impl Progress {
    pub fn from_events(events: impl IntoIterator<Item = String>) -> Self {
        Self { won: events.into_iter().collect() }
    }

    /// Whether the event was won (its check mark shows).
    pub fn is_won(&self, event: &str) -> bool {
        self.won.contains(event)
    }

    /// Notes a win. Returns whether it was new.
    pub fn record(&mut self, event: &str) -> bool {
        self.won.insert(event.to_owned())
    }

    /// Whether the event is open: it starts open, or a won event lists it.
    pub fn is_open(&self, board: &Board, entry: &Entry) -> bool {
        entry.open_at_start
            || board.entries.iter().any(|other| self.is_won(&other.event) && other.unlocks.contains(&entry.key))
    }

    /// The text of the file: `won = ["19.8.1", ...]`.
    fn to_toml(&self) -> String {
        let mut table = toml::Table::new();
        let won = self.won.iter().cloned().map(toml::Value::String).collect();
        table.insert("won".into(), toml::Value::Array(won));
        toml::to_string(&table).unwrap_or_default()
    }

    /// The progress a file's text holds; text that does not parse holds none.
    fn from_toml(text: &str) -> Self {
        let Ok(table) = text.parse::<toml::Table>() else { return Self::default() };
        let Some(list) = table.get("won").and_then(toml::Value::as_array) else { return Self::default() };
        Self::from_events(list.iter().filter_map(|v| v.as_str().map(str::to_owned)))
    }
}

/// The file the progress lives in: next to the config file; none when there is no per-user folder.
fn path() -> Option<PathBuf> {
    Some(crate::settings::Settings::config_path()?.parent()?.join(FILE))
}

/// The session's progress, read from the file on first use.
static SHARED: Mutex<Option<Progress>> = Mutex::new(None);

fn with_shared<T>(f: impl FnOnce(&mut Progress) -> T) -> T {
    let mut guard = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    let progress = guard.get_or_insert_with(|| {
        let text = path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        Progress::from_toml(&text)
    });
    f(progress)
}

/// A copy of the session's progress.
pub fn current() -> Progress {
    with_shared(|p| p.clone())
}

/// Notes a won event of the series and writes the file. Events outside the series (not `19.*`) are ignored.
pub fn record_win(event: &str) {
    if !event.starts_with("19.") {
        return;
    }
    let text = with_shared(|p| p.record(event).then(|| p.to_toml()));
    let Some(text) = text else { return };
    let Some(path) = path() else { return };
    if let Some(folder) = path.parent()
        && let Err(e) = std::fs::create_dir_all(folder)
    {
        log::warn!("challenge series: cannot make {}: {e}", folder.display());
        return;
    }
    if let Err(e) = std::fs::write(&path, text) {
        log::warn!("challenge series: cannot write {}: {e}", path.display());
    }
}

pub(crate) fn record_profile_reward(
    event: &str,
    handle: &crate::career::CareerHandle,
    profiles: Option<&crate::career::ProfileDir>,
) {
    if event != "19.8.31" {
        return;
    }
    let Some(profiles) = profiles else { return };
    handle.with_loaded(|career| {
        let before = career.settings.flags;
        career.settings.flags.insert(nfsmw_career::settings::Flags::QUICK_RACE_JUNKMAN);
        let alias = career.alias.clone();
        let bytes = career.profile_bytes();
        if let Err(error) = profiles.write(&alias, &bytes) {
            career.settings.flags = before;
            log::warn!("The My Cars Junkman reward could not be saved: {error:#}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::challenge_series::kinds::Kind;
    use crate::frontend::challenge_series::view::Crop;

    fn entry(key: u32, event: &str, open: bool, unlocks: &[u32]) -> Entry {
        Entry {
            key,
            name: event.into(),
            event: event.into(),
            kind: Kind::Race,
            tag: 0,
            goal: 0.0,
            open_at_start: open,
            unlocks: unlocks.to_vec(),
            crop: Crop::WHOLE,
            map_texture: 0,
        }
    }

    fn board() -> Board {
        Board {
            entries: vec![
                entry(1, "19.8.1", true, &[2]),
                entry(2, "19.8.2", false, &[3]),
                entry(3, "19.8.3", false, &[]),
            ],
        }
    }

    #[test]
    fn a_start_is_open_and_a_win_opens_what_it_lists() {
        let board = board();
        let mut progress = Progress::default();
        assert!(progress.is_open(&board, &board.entries[0]));
        assert!(!progress.is_open(&board, &board.entries[1]));
        progress.record("19.8.1");
        assert!(progress.is_open(&board, &board.entries[1]));
        assert!(!progress.is_open(&board, &board.entries[2]), "the second win is needed for the third");
    }

    #[test]
    fn a_win_is_noted_once() {
        let mut progress = Progress::default();
        assert!(progress.record("19.8.1"));
        assert!(!progress.record("19.8.1"));
        assert!(progress.is_won("19.8.1"));
    }

    #[test]
    fn the_file_text_round_trips_and_bad_text_holds_nothing() {
        let progress = Progress::from_events(["19.8.2".to_owned(), "19.8.1".to_owned()]);
        assert_eq!(Progress::from_toml(&progress.to_toml()), progress);
        assert_eq!(Progress::from_toml("not toml ["), Progress::default());
        assert_eq!(Progress::from_toml("won = 5"), Progress::default());
    }
}
