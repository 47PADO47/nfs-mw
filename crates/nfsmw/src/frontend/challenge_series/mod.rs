//! The challenge series screen (`ChallengeSeries.fng`, the original's `UIQRChallengeSeries`): a grid of the events of
//! bin 19 with a map of the selected one, its description, a lock or a check mark on each icon, and the dialogs that
//! launch it (are you sure, then manual or automatic). Spec: `docs/specs/challenge-series.md`.

pub(crate) mod board;
mod grid;
mod kinds;
pub(crate) mod maps;
mod progress;
mod texts;
mod view;

#[cfg(test)]
mod screen_tests;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use blackbox_feng::fe_hash_upper;
use blackbox_feng::ids::{BUTTON_PRESSED, PAD_BACK, PAD_BUTTON0, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};

#[cfg(test)]
pub(crate) use board::board;
pub use board::load_in_background;

/// The event the Mercedes SL 65 bonus car is unlocked by: "completing event number 68 in the Challenge Series" (label `0xBD8BAC91`).
const SPECIAL_EVENT: &str = "19.8.68";
/// The two events the series hides (the burger-chain race until its easter egg, the Black Edition race outside that edition):
/// finishing the series does not need them **[ours]**.
const OPTIONAL_EVENTS: [&str; 2] = ["19.8.31", "19.9.70"];

/// What the player has won in the series, for the bonus cars the car select unlocks with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Completion {
    /// Event 68 was won (`HasBeatenSpecialChallengeEvent`).
    pub special: bool,
    /// Every event was won (`HasBeatenChallengeSeries`).
    pub all: bool,
}

/// The wins of the series so far. Nothing is won while the events are not read yet.
pub fn completion() -> Completion {
    let Some(board) = board::board() else { return Completion::default() };
    let won = progress::current();
    let all =
        board.entries.iter().filter(|e| !OPTIONAL_EVENTS.contains(&e.event.as_str())).all(|e| won.is_won(&e.event));
    Completion { special: won.is_won(SPECIAL_EVENT), all: all && !board.entries.is_empty() }
}
pub(crate) use board::{map_texture, route_on_map};
pub use progress::record_win;

use super::confirm::{Confirm, Look};
use super::ids::{self, screen};
use super::logic::{Args, Command, Cx, ScreenLogic, wheel_as_rows};
use crate::settings::Transmission;
use crate::ui::Text;
use board::{Board, Entry};
use grid::{Grid, Step};
pub(crate) use maps::Maps;
use progress::Progress;
pub(crate) use view::{Crop, Zoom};

/// The grid is four icons wide and three high (`ArrayScrollerMenu(sd, 4, 3, true)`).
const WIDTH: usize = 4;
const HEIGHT: usize = 3;
const SLOTS: usize = WIDTH * HEIGHT;

/// The objects of the package, by name hash.
const MAP: u32 = 0x0F36_5871;
const EVENT_ICON: u32 = 0xA018_DE49;
const DESCRIPTION: u32 = 0x7B23_0D64;
const HEADER: u32 = 0x0790_CE49;
const EVENT_ID: u32 = 0xBBF9_70CD;
const LOCKED_TEXT: u32 = 0x6821_5623;
const NUMBER: u32 = 0x5A85_6A34;
const NUMBER_OF: u32 = 0x2D4D_22C8;
const BAR_BACKING: u32 = 0x688A_1551;
const BAR_HANDLE: u32 = 0x3FBB_7A4E;
/// The console footer (Accept, View Tutorial, Back with the pad's buttons): the package hides it on a PC, where the
/// footer of keys is its own two groups.
const CONSOLE_FOOTER: u32 = 0x84BC_F2B5;

/// Messages the screen posts to its package.
const TUTORIAL_HINT_ON: u32 = 0xB295_482E;
const TUTORIAL_HINT_OFF: u32 = 0x0F7B_54C7;
const LOCKED_ON: u32 = 0xC5DD_9D68;
const LOCKED_OFF: u32 = 0x3809_1FA1;
/// Shows the keys footer of the PC package (`Esc Back`, `2 View Tutorial`): `SHOW_HELP`.
const SHOW_HELP: u32 = 0xC5DB_4288;

/// Scripts of an icon: its idle look, the highlight and the way back, and a check mark's appearing and hiding.
const INIT: u32 = 0x0017_44B3;
const HIGHLIGHT: u32 = 0x249D_B7B7;
const UNHIGHLIGHT: u32 = 0x7AB5_521A;
const APPEAR: u32 = 0x5079_C8F8;
const HIDE: u32 = 0x0016_A259;

/// The pictures of a check mark: a padlock on a locked event, a tick on a won one (`STRINGHASH_LOCK`, `STRINGHASH_CHECK`).
const LOCK_TEXTURE: u32 = 0x0018_ED48;
const CHECK_TEXTURE: u32 = 0x028F_EADD;

/// The language labels the screen asks with: "Are you sure you want to launch this event?", "Select your
/// transmission.", and the two answers of the second question.
const LAUNCH_QUESTION: u32 = 0x77CF_03C5;
const TRANSMISSION_QUESTION: u32 = 0x6F54_01D1;
const MANUAL: u32 = 0x317D_3005;
const AUTO: u32 = 0x8CD5_32A0;

/// What a grid slot's icon shows now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Lit {
    #[default]
    Unknown,
    Idle,
    Highlighted,
}

pub struct ChallengeSeries {
    board: Option<Arc<Board>>,
    progress: Progress,
    grid: Grid,
    zoom: Zoom,
    /// The event the header and the map show; the header is written again only when it changes.
    shown: Option<usize>,
    lit: [Lit; SLOTS],
    maps: Maps,
    /// The package is leaving: nothing more is answered.
    leaving: bool,
}

impl Default for ChallengeSeries {
    fn default() -> Self {
        Self::new()
    }
}

impl ChallengeSeries {
    pub fn new() -> Self {
        Self {
            board: None,
            progress: Progress::default(),
            grid: Grid::new(WIDTH, HEIGHT, 0),
            zoom: Zoom::at(Crop::WHOLE),
            shown: None,
            lit: [Lit::Unknown; SLOTS],
            maps: Maps::default(),
            leaving: false,
        }
    }

    fn memory_key() -> String {
        screen::CHALLENGE_SERIES.to_owned()
    }

    /// The events, once they were read; the first frames may have none.
    fn take_board(&mut self, cx: &mut Cx) {
        if self.board.is_some() {
            return;
        }
        let Some(board) = board::board() else { return };
        self.progress = progress::current();
        self.adopt(cx, board);
    }

    /// Starts on a list of events: the selection the screen was left on, the map's first move and the map pictures.
    fn adopt(&mut self, cx: &mut Cx, board: Arc<Board>) {
        self.grid = Grid::new(WIDTH, HEIGHT, board.len());
        let remembered = cx.memory.get(&Self::memory_key()).copied().unwrap_or(0);
        self.grid.select(remembered);
        let first = board.entries.get(self.grid.current).map(|e| e.crop).unwrap_or(Crop::WHOLE);
        self.zoom = Zoom::at(Crop::WHOLE);
        self.zoom.go_to(first);
        self.maps.start(board.entries.iter().map(|e| e.map_texture).collect());
        self.board = Some(board);
        self.shown = None;
        self.refresh(cx);
    }

    /// A screen over the given events and wins, for the tests (the real one reads them from the install).
    #[cfg(test)]
    fn with(board: Arc<Board>, progress: Progress) -> Self {
        Self { board: Some(board), progress, ..Self::new() }
    }

    fn entry(&self) -> Option<&Entry> {
        self.board.as_ref()?.entries.get(self.grid.current)
    }

    fn locked(&self, entry: &Entry) -> bool {
        self.board.as_ref().is_some_and(|board| !self.progress.is_open(board, entry))
    }

    /// Writes everything the selection decides: the icons, the counter, the header, the description, the map, the
    /// lock panel, the tutorial hint and the scrollbar (`RefreshHeader`).
    fn refresh(&mut self, cx: &mut Cx) {
        let Some(board) = self.board.clone() else { return };
        self.refresh_slots(cx, &board);
        self.refresh_bar(cx);
        cx.text_group(NUMBER, &(self.grid.current + 1).to_string());
        cx.text_group(NUMBER_OF, &board.len().to_string());
        if self.shown == Some(self.grid.current) {
            return;
        }
        self.shown = Some(self.grid.current);
        let Some(entry) = board.entries.get(self.grid.current) else { return };
        self.refresh_event(cx, entry);
    }

    fn refresh_slots(&mut self, cx: &mut Cx, board: &Board) {
        for slot in 0..SLOTS {
            let name = fe_hash_upper(&format!("EVENT_ICON_{}", slot + 1));
            let check = fe_hash_upper(&format!("CHECK_{}", slot + 1));
            let Some(entry) = self.grid.index_of(slot).and_then(|i| board.entries.get(i)) else {
                cx.hide(name, true);
                cx.script(check, HIDE);
                self.lit[slot] = Lit::Unknown;
                continue;
            };
            cx.hide(name, false);
            if let Some(o) = cx.object(name) {
                cx.rt.set_texture(o, entry.kind.icon());
            }
            self.light(cx, slot, name);
            self.mark(cx, entry, check);
        }
        if let Some(o) = self.grid.slot().and_then(|s| cx.object(fe_hash_upper(&format!("EVENT_ICON_{}", s + 1)))) {
            cx.rt.focus_button(o);
        }
    }

    /// The icon's highlight or idle script, changed only when its state changes (`ArraySlot::Update`).
    fn light(&mut self, cx: &mut Cx, slot: usize, name: u32) {
        let selected = self.grid.slot() == Some(slot);
        let (state, script) = match (selected, self.lit[slot]) {
            (true, Lit::Highlighted) => return,
            (true, _) => (Lit::Highlighted, HIGHLIGHT),
            (false, Lit::Highlighted) => (Lit::Idle, UNHIGHLIGHT),
            (false, Lit::Idle) => return,
            (false, Lit::Unknown) => (Lit::Idle, INIT),
        };
        self.lit[slot] = state;
        cx.script(name, script);
    }

    /// A padlock on a locked event, a tick on a won one, nothing on the rest.
    fn mark(&self, cx: &mut Cx, entry: &Entry, check: u32) {
        let picture = match (self.locked(entry), self.progress.is_won(&entry.event)) {
            (true, _) => Some(LOCK_TEXTURE),
            (false, true) => Some(CHECK_TEXTURE),
            (false, false) => None,
        };
        let Some(texture) = picture else { return cx.script(check, HIDE) };
        if let Some(o) = cx.object(check) {
            cx.rt.set_texture(o, texture);
        }
        cx.script(check, APPEAR);
    }

    fn refresh_event(&mut self, cx: &mut Cx, entry: &Entry) {
        cx.hide(EVENT_ID, true);
        if let Some(o) = cx.object(HEADER) {
            cx.label(o, texts::header_label(entry.tag));
        }
        if let Some(o) = cx.object(EVENT_ICON) {
            cx.rt.set_texture(o, entry.kind.icon());
        }
        let template = Text::Game(texts::description_label(entry.tag)).resolve(cx.assets).unwrap_or_default();
        let goal = texts::goal_text(entry);
        cx.text_object(DESCRIPTION, &texts::fill(&template, &[goal.clone(), goal]));
        let message = if entry.kind.tutorial().is_some() { TUTORIAL_HINT_ON } else { TUTORIAL_HINT_OFF };
        cx.rt.post_to_package(cx.package, message);
        self.refresh_lock(cx, entry);
        self.show_map(cx, entry);
    }

    fn refresh_lock(&self, cx: &mut Cx, entry: &Entry) {
        if !self.locked(entry) {
            return cx.rt.post_to_package(cx.package, LOCKED_OFF);
        }
        cx.rt.post_to_package(cx.package, LOCKED_ON);
        let line = texts::locked_line(self.grid.current);
        let template = Text::Game(line.label).resolve(cx.assets).unwrap_or_default();
        let numbers: Vec<String> = line.numbers.iter().map(usize::to_string).collect();
        cx.text_object(LOCKED_TEXT, &texts::fill(&template, &numbers));
    }

    /// Swaps the map picture at once and lets the view ease to the event's crop (`SetMapLoaded`).
    fn show_map(&mut self, cx: &mut Cx, entry: &Entry) {
        self.zoom.go_to(entry.crop);
        self.place_map(cx, entry);
    }

    fn place_map(&self, cx: &mut Cx, entry: &Entry) {
        let Some(o) = cx.object(MAP) else { return };
        let loaded = cx.assets.texture(entry.map_texture).is_some();
        cx.rt.set_hidden(o, !loaded);
        if !loaded {
            return;
        }
        cx.rt.set_texture(o, entry.map_texture);
        cx.rt.set_uv(o, self.zoom.now().uv());
    }

    /// The scrollbar's handle: as tall as the window is of the list, at the window's place (`FEScrollBar::Update`).
    fn refresh_bar(&self, cx: &mut Cx) {
        let bar = self.grid.bar();
        let (Some(backing), Some(handle)) = (cx.object(BAR_BACKING), cx.object(BAR_HANDLE)) else { return };
        let shown = bar.rows > bar.view_rows;
        cx.rt.set_hidden(backing, !shown);
        cx.rt.set_hidden(handle, !shown);
        let (Some(at), Some(size), Some(width)) = (cx.rt.position(backing), cx.rt.size(backing), cx.rt.size(handle))
        else {
            return;
        };
        if !shown {
            return;
        }
        let top = at.y - size.y * 0.5;
        let height = bar.view_rows as f32 / bar.rows as f32 * size.y;
        let travel = (size.y - height) / (bar.rows - bar.view_rows).max(1) as f32;
        let y = top + (bar.top_row - 1) as f32 * travel;
        cx.rt.set_size_xy(handle, width.x, height);
        cx.rt.set_position_xy(handle, at.x, y + height * 0.5);
    }

    fn step(&mut self, cx: &mut Cx, step: Step) {
        if !self.grid.step(step) {
            return;
        }
        cx.memory.insert(Self::memory_key(), self.grid.current);
        self.refresh(cx);
    }

    /// Accept on an event: a locked one does nothing; an open one asks whether to launch it.
    fn press(&mut self, cx: &mut Cx) {
        cx.rt.set_input_enabled(cx.package, true);
        let Some(entry) = self.entry() else { return };
        if self.locked(entry) {
            return;
        }
        let question =
            Confirm::new(Text::Game(LAUNCH_QUESTION), Command::Confirm(self.transmission_question(cx, entry)))
                .looking(Look::Attention);
        cx.send(Command::Confirm(question));
    }

    /// Manual or automatic, each starting the event; the setting now has the focus.
    fn transmission_question(&self, cx: &Cx, entry: &Entry) -> Confirm {
        let start = |transmission| Command::StartChallenge { race: entry.name.clone(), transmission };
        let manual = cx.settings.transmission == Transmission::Manual;
        Confirm::choice(
            Text::Game(TRANSMISSION_QUESTION),
            (Text::Game(MANUAL), start(Transmission::Manual)),
            (Text::Game(AUTO), start(Transmission::Automatic)),
        )
        .looking(Look::Selection)
        .focusing_first(manual)
    }

    /// The tutorial button (`PAD_BUTTON4`): plays the event kind's movie, if it has one.
    fn tutorial(&mut self, cx: &mut Cx) {
        let Some(movie) = self.entry().and_then(|e| e.kind.tutorial()) else { return };
        cx.memory.insert(Self::memory_key(), self.grid.current);
        cx.send(Command::PlayTutorial {
            movie: movie.to_owned(),
            back: screen::CHALLENGE_SERIES.to_owned(),
            args: Args::default(),
        });
    }

    /// A click on an icon selects it and presses it (the package's mouse-down message is `BUTTON_PRESSED`).
    fn click_icon(&mut self, cx: &mut Cx, point: glam::Vec2) -> bool {
        let icons: Vec<Option<blackbox_feng::ObjectRef>> =
            (1..=SLOTS).map(|n| cx.object(fe_hash_upper(&format!("EVENT_ICON_{n}")))).collect();
        let found = icons.iter().flatten().copied().collect::<Vec<_>>();
        let Some(hit) = cx.rt.object_at(&found, point) else { return false };
        let Some(slot) = icons.iter().position(|o| *o == Some(hit)) else { return false };
        if !self.grid.pick(slot) {
            return false;
        }
        cx.memory.insert(Self::memory_key(), self.grid.current);
        self.refresh(cx);
        let before = cx.commands.len();
        self.press(cx);
        if cx.commands.len() != before {
            crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
        }
        true
    }

    fn leave(&mut self, cx: &mut Cx) {
        if self.leaving {
            return;
        }
        self.leaving = true;
        self.maps.release(cx.assets);
        cx.send(Command::Switch(screen::MAIN_MENU.into(), Args::default()));
    }
}

impl ScreenLogic for ChallengeSeries {
    fn start(&mut self, cx: &mut Cx) {
        cx.hide(CONSOLE_FOOTER, true);
        cx.rt.post_to_package(cx.package, SHOW_HELP);
        match self.board.clone() {
            Some(board) => self.adopt(cx, board),
            None => self.take_board(cx),
        }
    }

    fn resume(&mut self, cx: &mut Cx) {
        cx.rt.set_input_enabled(cx.package, true);
        self.lit = [Lit::Unknown; SLOTS];
        self.shown = None;
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        cx.hide(CONSOLE_FOOTER, true);
        self.take_board(cx);
        self.maps.poll(cx.assets);
        self.zoom.tick(dt);
        let Some(entry) = self.entry().cloned() else { return };
        self.place_map(cx, &entry);
    }

    fn click(&mut self, cx: &mut Cx, point: glam::Vec2) -> bool {
        !self.leaving && self.click_icon(cx, point)
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        wheel_as_rows(self, cx, steps);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if self.leaving {
            return;
        }
        match message {
            PAD_LEFT => self.step(cx, Step::Left),
            PAD_RIGHT => self.step(cx, Step::Right),
            PAD_UP => self.step(cx, Step::Up),
            PAD_DOWN => self.step(cx, Step::Down),
            BUTTON_PRESSED => self.press(cx),
            m if m == PAD_BUTTON0 + 4 => self.tutorial(cx),
            // The package plays its leave animation and sends Back when it is over.
            PAD_BACK => self.leave(cx),
            // The package sets its own focus when it has appeared, after `start`.
            ids::INIT_COMPLETE => self.refresh(cx),
            _ => {}
        }
    }
}
