//! The EA Trax jukebox (`EA_Trax_Jukebox.fng`): the licensed songs, four to a page, with the mode of each song (off, menus,
//! in game, both), the re-order and the order flag (`docs/specs/eatrax-jukebox.md`, sections 3 and 11). The choices are
//! saved as they are made (`settings::eatrax`); the radio reads them when it loads.
//!
//! Buttons (the package's messages): Up and Down move the highlight (with Re-Order on, they move the song); Left and
//! Right turn the page; Accept cycles the mode of the highlighted song; Re-Order (PAD_BUTTON5) turns re-order on and
//! off; L1 and R1 set the order to random or sequential; Prev (PAD_BUTTON4) moves the highlight up; Back leaves for the
//! audio options. The mapping of Re-Order, Prev and the order line is the port's reading (spec section 8).

use std::sync::OnceLock;

use blackbox_feng::ids::{
    PAD_ACCEPT, PAD_BACK, PAD_BUTTON0, PAD_DOWN, PAD_LEFT, PAD_LTRIGGER, PAD_RIGHT, PAD_RTRIGGER, PAD_UP,
};
use game_install::GameDir;
use glam::{Vec2, Vec3};
use nfsmw_data::music::Song;

use super::ids::{SCRIPT_HIGHLIGHT, SCRIPT_UNHIGHLIGHT, screen};
use super::logic::{Args, Category, Command, Cx, Pick, ScreenLogic, wheel_as_rows};
use crate::settings::eatrax::{Jukebox, Mode};
use crate::ui::UiAssets;
use crate::ui::theme::remastered::menu::layout::window_start;
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle, MenuValue};

/// The songs of the install's list, read once when the front end starts (`prime`). Empty when the install has none.
static SONGS: OnceLock<Vec<Song>> = OnceLock::new();

/// Reads the song list for the jukebox. Called once, when the front end is built; a failure leaves the jukebox empty.
pub fn prime(dir: &GameDir) {
    let songs = match read_songs(dir) {
        Ok(songs) => songs,
        Err(e) => {
            log::warn!("the EA Trax jukebox has no songs: {e}");
            Vec::new()
        }
    };
    let _ = SONGS.set(songs);
}

fn read_songs(dir: &GameDir) -> Result<Vec<Song>, String> {
    let db = nfsmw_data::shared::attributes(dir).map_err(|e| format!("ATTRIBUTES.BIN: {e:#}"))?;
    Ok(nfsmw_data::music::songs(&db))
}

/// Rows the package shows at once.
pub const ROWS: usize = 4;

/// The package's text objects by name hash (its dump, `nfsmw dump-screen`), one per row: the artist, the title (the
/// fourth row's title object is named as an album in the package, and is still the title), the album and the number.
const ARTIST: [u32; ROWS] = [0x6664_6FC4, 0x6664_6FC5, 0x6664_6FC6, 0x6664_6FC7];
const TITLE: [u32; ROWS] = [0x2890_C8AF, 0x2890_C8B0, 0x2890_C8B1, 0x2890_C8B2];
const ALBUM: [u32; ROWS] = [0x77BD_189E, 0x77BD_189F, 0x77BD_18A0, 0x77BD_18A1];
const NUMBER: [u32; ROWS] = [0xE454_E9A5, 0xE454_E9A6, 0xE454_E9A7, 0xE454_E9A8];
/// The footer labels the package draws, with the message each click on them sends. The package has no answer to a mouse
/// release on them, so the screen takes the click by the label's place (box centre: its origin, a little to the right; half
/// size below).
const FOOTER: [(u32, u32); 4] =
    [(0xD7A4_BD15, PAD_BACK), (0xB941_39F4, PAD_ACCEPT), (0x1F7F_2DE0, RE_ORDER), (0xD8A2_7BE6, PREV)];
const FOOTER_HALF: Vec2 = Vec2::new(40.0, 14.0);
/// The play-order line between the L1 and R1 arrows.
const ORDER_LINE: u32 = 0xCA74_A2FA;

/// The messages the jukebox answers, all from the package (the pad messages, and `PAD_BUTTON4` and `PAD_BUTTON5` for the
/// two keys the package sends with them).
const PREV: u32 = PAD_BUTTON0 + 4;
const RE_ORDER: u32 = PAD_BUTTON0 + 5;

/// What a press changed, so the screen knows what to redraw and save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Nothing,
    /// The highlight or the page moved.
    Moved,
    /// The jukebox changed; the screen redraws and saves it.
    Saved,
    /// Leave for the audio options.
    Leave,
}

/// The jukebox's state on the screen: the table, the highlight and the re-order switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub jukebox: Jukebox,
    pub selected: usize,
    pub reorder: bool,
}

impl State {
    pub fn new(jukebox: Jukebox) -> Self {
        Self { jukebox, selected: 0, reorder: false }
    }

    /// The page the highlight is on, from the first row.
    pub fn page_start(&self) -> usize {
        self.selected / ROWS * ROWS
    }

    /// The last page's first row.
    fn last_page_start(&self) -> usize {
        self.jukebox.len().saturating_sub(1) / ROWS * ROWS
    }

    /// Answers one message. Returns what changed.
    pub fn press(&mut self, message: u32) -> Action {
        match message {
            PAD_UP => self.step(-1),
            PAD_DOWN => self.step(1),
            PREV => self.select(-1),
            PAD_LEFT => self.page(-1),
            PAD_RIGHT => self.page(1),
            PAD_ACCEPT => self.cycle(),
            RE_ORDER => {
                self.reorder = !self.reorder;
                Action::Moved
            }
            PAD_LTRIGGER | PAD_RTRIGGER => {
                self.jukebox.random = !self.jukebox.random;
                Action::Saved
            }
            PAD_BACK => Action::Leave,
            _ => Action::Nothing,
        }
    }

    /// Up or down: moves the highlight, or the song when re-order is on.
    fn step(&mut self, delta: isize) -> Action {
        if !self.reorder {
            return self.select(delta);
        }
        if self.jukebox.is_empty() {
            return Action::Nothing;
        }
        let to = self.jukebox.move_by(self.selected, delta);
        if to == self.selected {
            return Action::Nothing;
        }
        self.selected = to;
        Action::Saved
    }

    /// Moves the highlight by `delta` rows, within the table.
    fn select(&mut self, delta: isize) -> Action {
        if self.jukebox.is_empty() {
            return Action::Nothing;
        }
        let last = self.jukebox.len() - 1;
        let to = self.selected.saturating_add_signed(delta).min(last);
        if to == self.selected {
            return Action::Nothing;
        }
        self.selected = to;
        Action::Moved
    }

    /// Turns the page by `delta` pages (not while re-ordering).
    fn page(&mut self, delta: isize) -> Action {
        if self.reorder || self.jukebox.is_empty() {
            return Action::Nothing;
        }
        let pages = self.last_page_start() / ROWS;
        let page = (self.page_start() / ROWS).saturating_add_signed(delta).min(pages);
        let to = page * ROWS;
        if to == self.page_start() {
            return Action::Nothing;
        }
        self.selected = to;
        Action::Moved
    }

    /// The mode button: the highlighted song's mode goes on to the next one.
    fn cycle(&mut self) -> Action {
        if self.jukebox.is_empty() {
            return Action::Nothing;
        }
        self.jukebox.cycle(self.selected);
        Action::Saved
    }
}

/// The row's number as the package shows it ("01").
pub fn number(pos: usize) -> String {
    format!("{:02}", pos + 1)
}

/// The title line of a row: the title, then the mode in brackets, with a mark on the highlighted row while re-ordering.
pub fn title_line(title: &str, mode: Mode, mark: bool) -> String {
    let prefix = if mark { "> " } else { "" };
    format!("{prefix}{title} [{}]", mode.label())
}

/// The order line's word.
pub fn order_word(random: bool) -> &'static str {
    if random { "Random" } else { "Sequential" }
}

/// Shows the jukebox and passes the package's messages to [`State`].
pub struct JukeboxLogic {
    songs: &'static [Song],
    state: State,
    /// Whether the Remastered look draws the playlist (`Cx::vertical_look`), and the song its window starts at.
    vertical: bool,
    top: usize,
}

impl JukeboxLogic {
    /// The jukebox over the install's songs, with the saved choices (or the defaults).
    pub fn new() -> Self {
        let songs: &'static [Song] = SONGS.get().map_or(&[], Vec::as_slice);
        let defaults = Jukebox::defaults_for(songs);
        Self { songs, state: State::new(Jukebox::load(&defaults)), vertical: false, top: 0 }
    }

    fn fill(&self, cx: &mut Cx) {
        let first = self.state.page_start();
        if self.songs.is_empty() {
            set_text(cx, ARTIST[0], "No EA Trax songs in this install");
        }
        for slot in 0..ROWS {
            let pos = first + slot;
            let entry = self.state.jukebox.entries().get(pos).copied();
            let song = entry.and_then(|e| self.songs.get(e.song));
            match (entry, song) {
                (Some(entry), Some(song)) => {
                    let selected = pos == self.state.selected;
                    set_text(cx, NUMBER[slot], &number(pos));
                    set_text(cx, ARTIST[slot], &song.artist);
                    let mark = selected && self.state.reorder;
                    set_text(cx, TITLE[slot], &title_line(&song.title, entry.mode, mark));
                    set_text(cx, ALBUM[slot], &song.album);
                    script(cx, TITLE[slot], if selected { SCRIPT_HIGHLIGHT } else { SCRIPT_UNHIGHLIGHT });
                }
                _ => {
                    for name in [NUMBER[slot], ARTIST[slot], TITLE[slot], ALBUM[slot]] {
                        set_text(cx, name, "");
                    }
                }
            }
        }
        set_text(cx, ORDER_LINE, order_word(self.state.jukebox.random));
    }

    /// Writes the choices to the file. A failure is logged; the screen keeps the choices for this session.
    fn save(&self) {
        if let Err(e) = self.state.jukebox.save() {
            log::warn!("the EA Trax jukebox could not be saved: {e}");
        }
    }
}

impl Default for JukeboxLogic {
    fn default() -> Self {
        Self::new()
    }
}

fn set_text(cx: &mut Cx, name: u32, text: &str) {
    let Some(o) = cx.object(name) else { return };
    cx.rt.set_text(o, text);
}

fn script(cx: &mut Cx, name: u32, which: u32) {
    cx.script(name, which);
}

impl ScreenLogic for JukeboxLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.vertical_look();
        self.fill(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
        self.top = window_start(self.top, self.state.selected, self.state.jukebox.len());
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if self.state.reorder || self.state.jukebox.is_empty() {
            return;
        }
        self.state.selected = row.min(self.state.jukebox.len() - 1);
        match pick {
            Pick::Hover => self.fill(cx),
            Pick::Press(..) => self.act_pointer(cx, PAD_ACCEPT),
        }
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let entries = self
            .state
            .jukebox
            .entries()
            .iter()
            .map(|entry| {
                let label = self
                    .songs
                    .get(entry.song)
                    .map_or_else(String::new, |song| format!("{} - {}", song.artist, song.title));
                MenuEntry {
                    label,
                    enabled: true,
                    value: MenuValue::Choice { text: entry.mode.label().to_owned(), arrows: false },
                    icon: None,
                }
            })
            .collect();
        let mut order = format!("{} order", order_word(self.state.jukebox.random));
        if self.state.reorder {
            order.push_str(", re-order on");
        }
        Some(MenuList {
            title: "EA Trax".into(),
            message: order,
            entries,
            selected: self.state.selected,
            first: self.top,
            hints: Vec::new(),
            style: MenuStyle::Sub,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        wheel_as_rows(self, cx, steps);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        self.act(cx, message);
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.vertical {
            return false;
        }
        let tree = cx.rt.tree(cx.package);
        for (name, message) in FOOTER {
            let Some(label) = tree.nodes.iter().find(|n| n.name_hash == name) else { continue };
            let centre = label.world.transform_point3(Vec3::ZERO).truncate() + Vec2::new(20.0, 0.0);
            if (point - centre).abs().cmple(FOOTER_HALF).all() {
                self.act_pointer(cx, message);
                return true;
            }
        }
        false
    }
}

impl JukeboxLogic {
    fn act_pointer(&mut self, cx: &mut Cx, message: u32) {
        let before = self.state.clone();
        let commands = cx.commands.len();
        self.act(cx, message);
        if self.state != before || cx.commands.len() != commands {
            crate::audio::menu::request(message);
        }
    }

    fn act(&mut self, cx: &mut Cx, message: u32) {
        match self.state.press(message) {
            Action::Nothing => {}
            Action::Moved => self.fill(cx),
            Action::Saved => {
                self.save();
                self.fill(cx);
            }
            Action::Leave => {
                self.save();
                let args = Args { options: true, category: Category::Audio, ..Args::default() };
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), args));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten songs: all on in the menus, the even ones in game too.
    fn state(songs: usize) -> State {
        let playable: Vec<(bool, bool)> = (0..songs).map(|n| (true, n % 2 == 0)).collect();
        State::new(Jukebox::defaults(&playable))
    }

    #[test]
    fn accept_cycles_the_highlighted_song_and_saves() {
        let mut s = state(10);
        assert_eq!(s.press(PAD_ACCEPT), Action::Saved);
        assert_eq!(s.jukebox.mode_of(0), Some(Mode::Off), "song 0 is both; the mode after both is off");
    }

    #[test]
    fn accept_on_an_empty_table_does_nothing() {
        let mut s = state(0);
        assert_eq!(s.press(PAD_ACCEPT), Action::Nothing);
        assert_eq!(s.press(PAD_DOWN), Action::Nothing);
    }

    #[test]
    fn up_and_down_move_the_highlight_within_the_table() {
        let mut s = state(10);
        assert_eq!(s.press(PAD_UP), Action::Nothing, "already at the top");
        assert_eq!(s.press(PAD_DOWN), Action::Moved);
        assert_eq!(s.selected, 1);
        for _ in 0..20 {
            s.press(PAD_DOWN);
        }
        assert_eq!(s.selected, 9, "stops at the last song");
    }

    #[test]
    fn the_arrows_turn_the_pages_of_four() {
        let mut s = state(10);
        assert_eq!(s.press(PAD_RIGHT), Action::Moved);
        assert_eq!(s.selected, 4);
        assert_eq!(s.press(PAD_RIGHT), Action::Moved);
        assert_eq!(s.selected, 8);
        assert_eq!(s.press(PAD_RIGHT), Action::Nothing, "the last page is already shown");
        assert_eq!(s.press(PAD_LEFT), Action::Moved);
        assert_eq!(s.selected, 4);
    }

    #[test]
    fn with_re_order_on_the_arrows_move_the_song_and_its_place_follows() {
        let mut s = state(10);
        s.press(PAD_DOWN);
        assert_eq!(s.press(RE_ORDER), Action::Moved);
        assert!(s.reorder);
        assert_eq!(s.press(PAD_UP), Action::Saved);
        assert_eq!(s.selected, 0);
        assert_eq!(s.jukebox.entries()[0].song, 1, "song 1 moved above song 0");
        assert_eq!(s.press(PAD_LEFT), Action::Nothing, "no page turn while re-ordering");
    }

    #[test]
    fn l1_and_r1_set_the_order_and_save() {
        let mut s = state(3);
        assert!(!s.jukebox.random);
        assert_eq!(s.press(PAD_LTRIGGER), Action::Saved);
        assert!(s.jukebox.random);
        assert_eq!(s.press(PAD_RTRIGGER), Action::Saved);
        assert!(!s.jukebox.random);
    }

    #[test]
    fn prev_moves_up_even_while_re_ordering() {
        let mut s = state(10);
        s.press(PAD_DOWN);
        s.press(PAD_DOWN);
        s.press(RE_ORDER);
        assert_eq!(s.press(PREV), Action::Moved);
        assert_eq!(s.selected, 1);
        assert_eq!(s.jukebox.entries()[2].song, 2, "Prev does not move a song");
    }

    #[test]
    fn back_leaves() {
        let mut s = state(3);
        assert_eq!(s.press(PAD_BACK), Action::Leave);
    }

    #[test]
    fn the_row_text_is_the_number_and_the_mode() {
        assert_eq!(number(0), "01");
        assert_eq!(number(11), "12");
        assert_eq!(title_line("Tilted", Mode::Both, false), "Tilted [Both]");
        assert_eq!(title_line("Tilted", Mode::Off, true), "> Tilted [Off]");
        assert_eq!(order_word(true), "Random");
        assert_eq!(order_word(false), "Sequential");
    }
}
