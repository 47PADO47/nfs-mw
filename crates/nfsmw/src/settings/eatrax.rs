//! The EA Trax jukebox: which licensed songs play in the menus and in game, in what order, and whether the order is
//! sequential or random (`docs/specs/eatrax-jukebox.md`, sections 7 and 11).
//!
//! The player's choices are kept in `eatrax_jukebox.toml` next to the config file, in the table order the player chose
//! (the original's profile table, read in table order). A missing or unreadable file is the defaults: every song keeps
//! the playability its `DefPlay` gives. The file is its own; the config file and its writer are not touched.

use std::path::PathBuf;

use toml::{Table, Value};

/// The file's name, next to the config file.
pub const FILE_NAME: &str = "eatrax_jukebox.toml";

/// The 28 bits the original's play-list masks hold.
#[cfg(test)]
const MASK_BITS: usize = 28;

/// Where one song plays: nowhere, the menus, the game, or both (the original's profile modes 0 to 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Menu,
    InGame,
    Both,
}

impl Mode {
    /// Whether the song plays in the menus (front-end list).
    pub fn menu(self) -> bool {
        matches!(self, Self::Menu | Self::Both)
    }

    /// Whether the song plays in game (in-game list).
    pub fn in_game(self) -> bool {
        matches!(self, Self::InGame | Self::Both)
    }

    /// The next mode when the player presses the mode button: off, menu, in game, both, and off again.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Menu,
            Self::Menu => Self::InGame,
            Self::InGame => Self::Both,
            Self::Both => Self::Off,
        }
    }

    /// The word the screen shows for the mode, in the current language.
    pub fn label(self) -> String {
        let id = match self {
            Self::Off => "options-off",
            Self::Menu => "options-jukebox-menu",
            Self::InGame => "options-jukebox-in-game",
            Self::Both => "options-jukebox-both",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }

    /// The number the file keeps (the original's profile value).
    fn code(self) -> i64 {
        match self {
            Self::Off => 0,
            Self::Menu => 1,
            Self::InGame => 2,
            Self::Both => 3,
        }
    }

    fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(Self::Off),
            1 => Some(Self::Menu),
            2 => Some(Self::InGame),
            3 => Some(Self::Both),
            _ => None,
        }
    }

    /// The mode a song's `DefPlay` gives: `playable` is (front end, in game).
    fn default_for((menu, game): (bool, bool)) -> Self {
        match (menu, game) {
            (true, true) => Self::Both,
            (true, false) => Self::Menu,
            (false, true) => Self::InGame,
            (false, false) => Self::Off,
        }
    }
}

/// One song in the table: its number in the song list and its mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub song: usize,
    pub mode: Mode,
}

/// The jukebox's state: the table in play order, and the order flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jukebox {
    entries: Vec<Entry>,
    /// Random order (the "Squential" line set to random); false is sequential, the table order.
    pub random: bool,
}

impl Jukebox {
    /// The defaults: every song in number order, with the mode its playability gives. `playable[n]` is whether song `n`
    /// plays (front end, in game).
    pub fn defaults(playable: &[(bool, bool)]) -> Self {
        let entries =
            playable.iter().enumerate().map(|(song, &p)| Entry { song, mode: Mode::default_for(p) }).collect();
        Self { entries, random: false }
    }

    /// The defaults for a song list: each song with the mode its `DefPlay` gives.
    pub fn defaults_for(songs: &[nfsmw_data::music::Song]) -> Self {
        let playable: Vec<(bool, bool)> =
            songs.iter().map(|s| (s.playability.front_end(), s.playability.in_game())).collect();
        Self::defaults(&playable)
    }

    /// The table in play order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The number of songs in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has no song.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The mode of song `song`, when it is in the table.
    #[cfg(test)]
    pub fn mode_of(&self, song: usize) -> Option<Mode> {
        self.entries.iter().find(|e| e.song == song).map(|e| e.mode)
    }

    /// Advances the mode of the entry at table position `pos` (the mode button). Does nothing past the table.
    pub fn cycle(&mut self, pos: usize) {
        if let Some(entry) = self.entries.get_mut(pos) {
            entry.mode = entry.mode.next();
        }
    }

    /// Moves the entry at `pos` by `delta` places (re-order). Returns the position it ends at; a move past either end
    /// stays where it is.
    pub fn move_by(&mut self, pos: usize, delta: isize) -> usize {
        let len = self.entries.len();
        let Some(to) = pos.checked_add_signed(delta).filter(|&t| t < len && pos < len) else { return pos };
        self.entries.swap(pos, to);
        to
    }

    /// The bit mask of list `list` (0 the menus, 1 in game): bit `n` is set when song `n` plays in that list. Songs past
    /// the 28 bits are not in the mask.
    #[cfg(test)]
    pub fn mask(&self, list: usize) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.song < MASK_BITS && in_list(e.mode, list))
            .fold(0u32, |mask, e| mask | 1 << e.song)
    }

    /// The songs of list `list` in table order: the order the sequential play takes them.
    pub fn order(&self, list: usize) -> Vec<usize> {
        self.entries.iter().filter(|e| in_list(e.mode, list)).map(|e| e.song).collect()
    }

    /// The file's text: the order flag and the table as `[song, mode]` pairs.
    pub fn to_toml(&self) -> String {
        let pairs: Vec<String> = self.entries.iter().map(|e| format!("[{}, {}]", e.song, e.mode.code())).collect();
        format!("random = {}\nsongs = [{}]\n", self.random, pairs.join(", "))
    }

    /// Reads the file's text over `defaults`. Entries that name a song out of range, or a song twice, are dropped; a
    /// bad mode or order flag keeps the default. Songs the file leaves out follow in default order with their default
    /// modes, so a file written by an older song list still covers every song.
    pub fn parse(text: &str, defaults: &Jukebox, origin: &str) -> Self {
        let table: Table = match text.parse() {
            Ok(table) => table,
            Err(e) => {
                log::warn!("ignoring {origin}: {e}");
                return defaults.clone();
            }
        };
        let random = match table.get("random") {
            None => defaults.random,
            Some(Value::Boolean(b)) => *b,
            Some(_) => {
                log::warn!("{origin}: random must be true or false; using sequential");
                defaults.random
            }
        };
        let mut entries: Vec<Entry> = Vec::with_capacity(defaults.len());
        let pairs = table.get("songs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
        for pair in pairs {
            let Some(entry) = entry_of(pair, defaults.len()) else {
                log::warn!("{origin}: a song entry {pair} is not [song, mode]; dropped");
                continue;
            };
            if entries.iter().all(|e| e.song != entry.song) {
                entries.push(entry);
            }
        }
        for entry in &defaults.entries {
            if entries.iter().all(|e| e.song != entry.song) {
                entries.push(*entry);
            }
        }
        Self { entries, random }
    }

    /// The saved jukebox, or `defaults` when there is no file (or no per-user config folder).
    pub fn load(defaults: &Jukebox) -> Self {
        let Some(path) = path() else { return defaults.clone() };
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text, defaults, &path.display().to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => defaults.clone(),
            Err(e) => {
                log::warn!("cannot read {}: {e}", path.display());
                defaults.clone()
            }
        }
    }

    /// Writes the file next to the config file. Returns the path written.
    pub fn save(&self) -> std::io::Result<PathBuf> {
        let path = path().ok_or_else(|| std::io::Error::other("there is no per-user config folder"))?;
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(&path, self.to_toml())?;
        Ok(path)
    }
}

fn in_list(mode: Mode, list: usize) -> bool {
    if list == 0 { mode.menu() } else { mode.in_game() }
}

/// One `[song, mode]` pair of the file, when it is one and names a song in range.
fn entry_of(pair: &Value, songs: usize) -> Option<Entry> {
    let items = pair.as_array()?;
    let [Value::Integer(song), Value::Integer(code)] = items.as_slice() else { return None };
    let song = usize::try_from(*song).ok().filter(|&n| n < songs)?;
    let mode = Mode::from_code(*code)?;
    Some(Entry { song, mode })
}

/// The file's path: next to the config file. `None` when there is no per-user config folder.
pub fn path() -> Option<PathBuf> {
    let config = crate::settings::Settings::config_path()?;
    Some(config.parent()?.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Five songs: front end only, both, in game only, nowhere, both.
    fn defaults() -> Jukebox {
        Jukebox::defaults(&[(true, false), (true, true), (false, true), (false, false), (true, true)])
    }

    #[test]
    fn the_defaults_follow_the_playability() {
        let jukebox = defaults();
        let modes: Vec<Mode> = jukebox.entries().iter().map(|e| e.mode).collect();
        assert_eq!(modes, [Mode::Menu, Mode::Both, Mode::InGame, Mode::Off, Mode::Both]);
        assert!(!jukebox.random);
    }

    #[test]
    fn each_list_mask_holds_the_songs_of_its_modes() {
        let jukebox = defaults();
        assert_eq!(jukebox.mask(0), 0b1_0011, "menus: songs 0, 1, 4");
        assert_eq!(jukebox.mask(1), 0b1_0110, "in game: songs 1, 2, 4");
    }

    #[test]
    fn the_mode_button_cycles_off_menu_game_both() {
        let mut jukebox = defaults();
        jukebox.cycle(3);
        assert_eq!(jukebox.mode_of(3), Some(Mode::Menu));
        jukebox.cycle(3);
        jukebox.cycle(3);
        assert_eq!(jukebox.mode_of(3), Some(Mode::Both));
        jukebox.cycle(3);
        assert_eq!(jukebox.mode_of(3), Some(Mode::Off));
        jukebox.cycle(99);
        assert_eq!(jukebox.len(), 5, "a position past the table changes nothing");
    }

    #[test]
    fn sequential_order_is_the_table_order_of_each_list() {
        let mut jukebox = defaults();
        jukebox.move_by(4, -3);
        assert_eq!(jukebox.entries()[1].song, 4, "the moved song is now second");
        assert_eq!(jukebox.order(0), vec![0, 4, 1]);
        assert_eq!(jukebox.order(1), vec![4, 2, 1]);
    }

    #[test]
    fn a_move_past_an_end_stays_put() {
        let mut jukebox = defaults();
        assert_eq!(jukebox.move_by(0, -1), 0);
        assert_eq!(jukebox.move_by(4, 1), 4);
        assert_eq!(jukebox.move_by(2, 1), 3);
        assert_eq!(jukebox.entries()[3].song, 2);
    }

    #[test]
    fn the_file_round_trips() {
        let mut jukebox = defaults();
        jukebox.cycle(0);
        jukebox.move_by(2, -2);
        jukebox.random = true;
        let text = jukebox.to_toml();
        assert_eq!(Jukebox::parse(&text, &defaults(), "test"), jukebox);
    }

    #[test]
    fn a_bad_file_keeps_the_defaults() {
        assert_eq!(Jukebox::parse("songs = [[0", &defaults(), "test"), defaults());
    }

    #[test]
    fn the_file_drops_out_of_range_and_repeated_songs_and_covers_the_rest() {
        let text = "random = true\nsongs = [[4, 3], [9, 1], [4, 0], [2, 7], [1, 1]]\n";
        let jukebox = Jukebox::parse(text, &defaults(), "test");
        assert!(jukebox.random);
        let songs: Vec<usize> = jukebox.entries().iter().map(|e| e.song).collect();
        assert_eq!(songs, [4, 1, 0, 2, 3], "the file's songs first, then the others in default order");
        assert_eq!(jukebox.mode_of(4), Some(Mode::Both), "the first entry of a song wins");
        assert_eq!(jukebox.mode_of(2), Some(Mode::InGame), "a mode out of range keeps the default");
    }

    #[test]
    fn a_non_boolean_order_flag_is_sequential() {
        let jukebox = Jukebox::parse("random = 1\n", &defaults(), "test");
        assert!(!jukebox.random);
    }

    #[test]
    fn the_radio_mask_has_its_28_bits() {
        let jukebox = Jukebox::defaults(&[(true, true); 30]);
        assert_eq!(jukebox.mask(0), (1u32 << 28) - 1);
    }
}
