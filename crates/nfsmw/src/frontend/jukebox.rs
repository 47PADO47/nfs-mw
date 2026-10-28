//! The EA Trax jukebox (`EA_Trax_Jukebox.fng`): the licensed songs, four to a page, with the mode of each song (off, menus,
//! in game, both), the re-order and the order flag (`docs/specs/eatrax-jukebox.md`, sections 3 and 11). The choices are
//! saved as they are made (`settings::eatrax`); the radio reads them when it loads.
//!
//! Buttons (the package's messages): Up and Down move the highlight (with Re-Order on, they move the song); Left and
//! Right change the mode of the highlighted song; Accept exits; Grab (PAD_BUTTON5) turns re-order on and
//! off; L1 and R1 set the order to random or sequential; Preview (PAD_BUTTON4) plays the selected song; Back leaves for
//! the audio options. The recorded modded reference supplies the Preview/Grab controls (spec section 12).

use std::sync::OnceLock;

use blackbox_feng::ids::{
    PAD_ACCEPT, PAD_BACK, PAD_BUTTON0, PAD_DOWN, PAD_LEFT, PAD_LTRIGGER, PAD_RIGHT, PAD_RTRIGGER, PAD_UP,
};
use game_install::GameDir;
use glam::Vec2;
use nfsmw_data::music::Song;

use super::ids::{SCRIPT_HIGHLIGHT, SCRIPT_UNHIGHLIGHT, screen};
use super::logic::{Args, Category, Command, Cx, Footer, ListScreen, Pick, ScreenLogic, wheel_as_rows};
use crate::i18n::t;
use crate::settings::eatrax::{Jukebox, Mode};
use crate::ui::theme::remastered::menu::layout::window_start;
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle, MenuValue};
use crate::ui::{Text, UiAssets};

/// The songs of the install's list, read once when the front end starts (`prime`). Empty when the install has none.
static SONGS: OnceLock<Vec<Song>> = OnceLock::new();

impl super::flow::Frontend {
    /// Preview ownership ends even when an external screen switch clears the jukebox.
    pub fn jukebox_active(&self) -> bool {
        self.screens.top().is_some_and(|name| name.eq_ignore_ascii_case(screen::EA_TRAX_JUKEBOX))
    }
}

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
const MODE: [u32; ROWS] = [0xF3EB_DC4E, 0xF3EB_DC4F, 0xF3EB_DC50, 0xF3EB_DC51];
const ALBUM: [u32; ROWS] = [0x77BD_189E, 0x77BD_189F, 0x77BD_18A0, 0x77BD_18A1];
const NUMBER: [u32; ROWS] = [0xE454_E9A5, 0xE454_E9A6, 0xE454_E9A7, 0xE454_E9A8];
/// The play-order line between the L1 and R1 arrows.
const ORDER_LINE: u32 = 0xCA74_A2FA;

/// The messages the jukebox answers, all from the package (the pad messages, and `PAD_BUTTON4` and `PAD_BUTTON5` for the
/// two keys the package sends with them).
const PREVIEW: u32 = PAD_BUTTON0 + 4;
const RE_ORDER: u32 = PAD_BUTTON0 + 5;

/// What a press changed, so the screen knows what to redraw and save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Nothing,
    /// The highlight or the page moved.
    Moved,
    /// The jukebox changed; the screen redraws and saves it.
    Saved,
    /// Preview this song's index in the installed list, independently of table order and enabled mode.
    Preview(usize),
    /// Leave for the audio options.
    Leave,
}

/// The jukebox's state on the screen: the table, the highlight and the re-order switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub jukebox: Jukebox,
    pub selected: usize,
    pub first: usize,
    pub reorder: bool,
}

impl State {
    pub fn new(jukebox: Jukebox) -> Self {
        Self { jukebox, selected: 0, first: 0, reorder: false }
    }

    /// The first visible row of the four-row window.
    pub fn page_start(&self) -> usize {
        self.first
    }

    fn follow(&mut self) {
        if self.selected < self.first {
            self.first = self.selected;
        }
        if self.selected >= self.first + ROWS {
            self.first = self.selected + 1 - ROWS;
        }
    }

    /// Answers one message. Returns what changed.
    pub fn press(&mut self, message: u32) -> Action {
        match message {
            PAD_UP => self.step(-1),
            PAD_DOWN => self.step(1),
            PREVIEW => {
                self.jukebox.entries().get(self.selected).map_or(Action::Nothing, |entry| Action::Preview(entry.song))
            }
            PAD_LEFT => self.cycle(-1),
            PAD_RIGHT => self.cycle(1),
            PAD_ACCEPT => Action::Leave,
            RE_ORDER => {
                if self.jukebox.is_empty() {
                    return Action::Nothing;
                }
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
        self.follow();
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
        self.follow();
        Action::Moved
    }

    /// Changes the selected mode in either direction.
    fn cycle(&mut self, delta: isize) -> Action {
        if self.reorder || self.jukebox.is_empty() {
            return Action::Nothing;
        }
        let count = match delta < 0 {
            true => 3,
            false => 1,
        };
        for _ in 0..count {
            self.jukebox.cycle(self.selected);
        }
        Action::Saved
    }
}

/// The row's number as the package shows it ("01").
pub fn number(pos: usize) -> String {
    format!("{:02}", pos + 1)
}

/// The title line, marked on the highlighted row while re-ordering.
pub fn title_line(title: &str, _mode: &str, mark: bool) -> String {
    let prefix = if mark { "> " } else { "" };
    format!("{prefix}{title}")
}

/// The installed jukebox's mode labels: Off, Menu, Racing and All.
pub fn mode_text(mode: Mode) -> String {
    mode.label()
}

/// The order line's word: the game's "Random" or "Sequential" (`JUKEBOX_RANDOM`, `JUKEBOX_INORDER`).
pub fn order_word(random: bool) -> Text {
    Text::Game(if random { 0xA9C9_C8F7 } else { 0x4CA3_6B89 })
}

/// Shows the jukebox and passes the package's messages to [`State`].
pub struct JukeboxLogic {
    songs: &'static [Song],
    state: State,
    /// Whether a list look draws the playlist (`Cx::screen_list_look`), and the song its window starts at.
    vertical: bool,
    top: usize,
    leaving: bool,
}

impl JukeboxLogic {
    /// The jukebox over the install's songs, with the saved choices (or the defaults).
    pub fn new() -> Self {
        let songs: &'static [Song] = SONGS.get().map_or(&[], Vec::as_slice);
        let defaults = Jukebox::defaults_for(songs);
        Self { songs, state: State::new(Jukebox::load(&defaults)), vertical: false, top: 0, leaving: false }
    }

    fn fill(&self, cx: &mut Cx) {
        let first = self.state.page_start();
        if self.songs.is_empty() {
            set_text(cx, ARTIST[0], &t!("menus-jukebox-no-songs"));
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
                    set_text(cx, TITLE[slot], &title_line(&song.title, &mode_text(entry.mode), mark));
                    set_text(cx, ALBUM[slot], &song.album);
                    set_text(cx, MODE[slot], &mode_text(entry.mode));
                    script(cx, TITLE[slot], if selected { SCRIPT_HIGHLIGHT } else { SCRIPT_UNHIGHLIGHT });
                }
                _ => {
                    for name in [NUMBER[slot], ARTIST[slot], TITLE[slot], ALBUM[slot], MODE[slot]] {
                        set_text(cx, name, "");
                    }
                }
            }
        }
        set_text(cx, ORDER_LINE, &order_word(self.state.jukebox.random).show());
        self.scrollbar(cx);
    }

    fn scrollbar(&self, cx: &mut Cx) {
        let tree = cx.rt.tree(cx.package);
        let Some(root) = tree.nodes.iter().find(|n| n.name_hash == 0xA808_E057).map(|n| n.index) else { return };
        let object = |hash| {
            tree.nodes
                .iter()
                .find(|n| n.parent == Some(root) && n.name_hash == hash)
                .map(|n| blackbox_feng::ObjectRef { package: cx.package, index: n.index })
        };
        let (Some(track), Some(handle)) = (object(0x688A_1551), object(0x3FBB_7A4E)) else { return };
        let (Some(pos), Some(size), Some(handle_size)) = (cx.rt.position(track), cx.rt.size(track), cx.rt.size(handle))
        else {
            return;
        };
        let range = (size.y.abs() - handle_size.y.abs()).max(0.0);
        let last = self.state.jukebox.len().saturating_sub(ROWS);
        let fraction = if last == 0 { 0.0 } else { self.state.first as f32 / last as f32 };
        cx.rt.set_position_xy(handle, pos.x, pos.y - range * 0.5 + range * fraction);
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
        self.vertical = cx.screen_list_look(ListScreen::Jukebox);
        self.fill(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.screen_list_look(ListScreen::Jukebox);
        self.top = window_start(self.top, self.state.selected, self.state.jukebox.len());
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn footer(&self) -> Footer {
        Footer::Jukebox { reorder: self.state.reorder, has_songs: !self.state.jukebox.is_empty() }
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if self.leaving || self.state.reorder || self.state.jukebox.is_empty() {
            return;
        }
        self.state.selected = row.min(self.state.jukebox.len() - 1);
        self.state.follow();
        match pick {
            Pick::Hover => self.fill(cx),
            Pick::Press(..) => self.act_pointer(cx, PAD_RIGHT),
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
                    value: MenuValue::Choice { text: mode_text(entry.mode), arrows: false },
                    icon: None,
                }
            })
            .collect();
        let order = match self.state.jukebox.random {
            true => t!("menus-jukebox-order-random"),
            false => t!("menus-jukebox-order-sequential"),
        };
        let message = match self.state.reorder {
            true => t!("menus-jukebox-reordering", order = order),
            false => order,
        };
        Some(MenuList {
            // The name of the game's licensed music, the same in every language.
            title: "EA Trax".into(),
            message,
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
        if self.leaving {
            return true;
        }
        // The presentation hides these native PC footer groups. Consume their raw targets so
        // the generic pointer fallback cannot activate an invisible package action.
        let Some(target) = cx.rt.mouse_target_at(cx.package, point, super::ids::MOUSE_LEFT_RELEASED) else {
            return false;
        };
        let Some(object) = cx.rt.package(cx.package).and_then(|p| p.objects.get(target.index)) else {
            return false;
        };
        [0x01BC_FC29, 0xD646_3100, 0x73DC_B662, 0xC0A3_2823].contains(&object.name_hash)
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
        if self.leaving {
            if message == super::ids::EXIT_COMPLETE {
                self.return_to_categories(cx);
            }
            return;
        }
        if message == PAD_ACCEPT && !self.vertical {
            self.leaving = true;
            return;
        }
        match self.state.press(message) {
            Action::Nothing => {}
            Action::Moved => self.fill(cx),
            Action::Saved => {
                self.save();
                cx.send(Command::JukeboxChanged(self.state.jukebox.clone()));
                self.fill(cx);
            }
            Action::Preview(song) => cx.send(Command::JukeboxPreview(Some(song))),
            Action::Leave => self.return_to_categories(cx),
        }
    }

    fn return_to_categories(&self, cx: &mut Cx) {
        self.save();
        cx.send(Command::JukeboxPreview(None));
        cx.send(Command::JukeboxChanged(self.state.jukebox.clone()));
        let args = Args { options: true, category: Category::Audio, ..Args::default() };
        cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), args));
    }
}

#[cfg(test)]
#[path = "jukebox_tests.rs"]
mod tests;
