//! The three reputation sheets behind the Blacklist page: race events (`SafeHouseRaceSheet.fng`), milestones
//! (`SafeHouseMilestones.fng`) and bounty (`SafeHouseBounty.fng`). `docs/specs/career-rapsheet-2.md` section 5 has the
//! original's classes; `docs/specs/career-rep-sheets.md` has what the rewrite does with them.
//!
//! The three packages are one layout: a title, a page counter ("3 / 9"), a map picture and a panel for the selected entry.
//! The original lets the player walk the entries with the pad (or by the 12 icon buttons), Accept asks a question and, on
//! Yes, leaves the menus for the entry: a race starts, a milestone or a bounty place puts the car on its spawn point.
//! Here a [`Page`] is the rows and what they show, and [`Sheet`] is the walk, the question and the leave. Back, and the
//! mouse, go as on the overview: Back returns to it, at the rival the sheet was opened for.

mod bounty;
#[cfg(test)]
mod install_tests;
mod milestones;
mod race;
#[cfg(test)]
mod tests;

use blackbox_feng::ids::{PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_feng::runtime::pad;
use glam::Vec2;

use super::reputation_overview;
use crate::frontend::challenge_series::{Crop, Maps, Zoom};
use crate::frontend::confirm::Confirm;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, Footer, ListScreen, Pick, ScreenLogic};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle, MenuValue};
use crate::ui::{Text, UiAssets};

/// Objects every sheet package has, by name hash: the page counter ("n / m": the primary strings are static, the shadows
/// carry the numbers), the stand-in event id and the map picture. The track select of the custom race shares them.
const COUNTER_NOW: u32 = 0x6F25_A248;
const COUNTER_OF: u32 = 0xB203_7BDC;
const COUNTER_STATIC: [u32; 3] = [0x5B80_5EE5, 0x2E48_1779, 0xF658_6F97];
const EVENT_ID: u32 = 0xBBF9_70CD;
const MAP: u32 = 0x0F36_5871;
/// The 12 icon buttons' group (race and milestone and bounty sheets alike). The rewrite walks the entries with the pad, so
/// the buttons stay out of the way (they would take the focus and answer nothing).
const ICON_GRID: u32 = 0xCA9F_A6F5;
/// The scroll arrows at the left of the panel: up to the entry before, down to the one after.
const ARROW_UP: u32 = 0x4449_69FD;
const ARROW_DOWN: u32 = 0x4449_69FE;

/// Every object of the package of `kind` the sheet and its page name (a test checks them against the install).
#[cfg(test)]
pub(super) fn used_objects(kind: Kind) -> Vec<u32> {
    let mut all = vec![COUNTER_NOW, COUNTER_OF, MAP, ICON_GRID, ARROW_UP, ARROW_DOWN];
    // The milestones package has no stand-in event id.
    if kind != Kind::Milestones {
        all.push(EVENT_ID);
    }
    all.extend(COUNTER_STATIC);
    all.extend(match kind {
        Kind::Race => race::used(),
        Kind::Milestones => milestones::used(),
        Kind::Bounty => bounty::used(),
    });
    all
}

/// Where an entry of a sheet stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not open yet: Accept does nothing.
    Locked,
    /// Open and not done: Accept asks to go.
    Open,
    /// Done (a race won, a milestone banked). A race can still be run again.
    Done,
}

/// What a Yes on the question does, as a command, and the question's language label.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub label: u32,
    pub then: Command,
}

/// The rows of one sheet and how they are shown. The data are read when the sheet opens.
pub trait Page: Send + Sync {
    /// How many entries.
    fn len(&self) -> usize;
    fn status(&self, at: usize) -> Status;
    /// The entry's name in the vertical list.
    fn name(&self, at: usize) -> String;
    /// The text to the right of the name in the vertical list.
    fn summary(&self, at: usize) -> String;
    /// The language label of the vertical list's title.
    fn heading(&self) -> u32;
    /// Writes the entry's panel into the package.
    fn fill(&self, cx: &mut Cx, at: usize);
    /// The question Accept asks on this entry, `None` when Accept does nothing on it.
    fn ask(&self, at: usize) -> Option<Ask>;
    /// The map picture of the entry and the part of it to show, when the sheet has maps.
    fn map(&self, _at: usize) -> Option<(u32, Crop)> {
        None
    }
    /// Every map picture the sheet may show (the sheet reads them on a worker thread).
    fn map_textures(&self) -> Vec<u32> {
        Vec::new()
    }
    /// The entry to start on.
    fn first(&self) -> usize {
        0
    }
}

/// Which sheet a package is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Race,
    Milestones,
    Bounty,
}

impl Kind {
    /// The kind of the package called `name` (lower case).
    pub fn of(name: &str) -> Option<Kind> {
        [
            (Kind::Race, screen::SAFE_HOUSE_RACE_SHEET),
            (Kind::Milestones, screen::SAFE_HOUSE_MILESTONES),
            (Kind::Bounty, screen::SAFE_HOUSE_BOUNTY),
        ]
        .into_iter()
        .find(|(_, package)| package.to_ascii_lowercase() == name)
        .map(|(kind, _)| kind)
    }
}

/// One step of a walk: the entry after `at` in `by` (positive forward), wrapping round the `len` entries.
pub fn walk(at: usize, by: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (at as isize + by).rem_euclid(len as isize) as usize
}

/// The logic of the sheet package called `name` (lower case), or `None` when `name` is none of the three.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    let kind = Kind::of(name)?;
    // Outside a career there is no Blacklist to show.
    super::blacklist::current_bin()?;
    Some(Box::new(Sheet::new(kind, args)))
}

/// A sheet on screen.
struct Sheet {
    kind: Kind,
    /// The bin whose entries are shown: the rival the overview was showing.
    bin: u8,
    page: Box<dyn Page>,
    at: usize,
    maps: Maps,
    zoom: Zoom,
    shown_map: Option<usize>,
    vertical: bool,
    /// What the sheet was opened with, for the screens it goes back to.
    args: Args,
}

impl Sheet {
    fn new(kind: Kind, args: Args) -> Sheet {
        let current = super::blacklist::current_bin().unwrap_or(nfsmw_career::settings::FIRST_RIVAL_STAGE);
        let bin = reputation_overview::first_shown(args.rival, reputation_overview::lowest(current));
        let page = open_page(kind, bin);
        let at = page.first();
        Sheet {
            kind,
            bin,
            page,
            at,
            maps: Maps::default(),
            zoom: Zoom::at(Crop::WHOLE),
            shown_map: None,
            vertical: false,
            args,
        }
    }

    fn len(&self) -> usize {
        self.page.len()
    }

    fn step(&mut self, cx: &mut Cx, by: isize) {
        if self.len() < 2 {
            return;
        }
        self.at = walk(self.at, by, self.len());
        self.refresh(cx);
    }

    /// Writes the counter and the selected entry's panel.
    fn refresh(&mut self, cx: &mut Cx) {
        for hidden in COUNTER_STATIC {
            cx.hide(hidden, true);
        }
        cx.hide(EVENT_ID, true);
        cx.hide(ICON_GRID, true);
        let now = if self.len() == 0 { 0 } else { self.at + 1 };
        cx.text_object(COUNTER_NOW, &now.to_string());
        cx.text_object(COUNTER_OF, &self.len().to_string());
        if self.len() > 0 {
            self.page.fill(cx, self.at);
        }
        self.place_map(cx);
    }

    fn place_map(&mut self, cx: &mut Cx) {
        let Some(map) = cx.object(MAP) else { return };
        let Some((texture, crop)) = self.page.map(self.at).filter(|_| self.len() > 0) else {
            return cx.rt.set_hidden(map, true);
        };
        if self.shown_map != Some(self.at) {
            self.shown_map = Some(self.at);
            self.zoom.go_to(crop);
        }
        let loaded = cx.assets.texture(texture).is_some();
        cx.rt.set_hidden(map, !loaded);
        if !loaded {
            return;
        }
        cx.rt.set_texture(map, texture);
        cx.rt.set_uv(map, self.zoom.now().uv());
    }

    /// Accept: the question of the entry, when it has one.
    fn accept(&mut self, cx: &mut Cx) {
        if self.len() == 0 {
            return;
        }
        let Some(ask) = self.page.ask(self.at) else { return };
        self.maps.release(cx.assets);
        cx.send(Command::Confirm(Confirm::new(Text::Game(ask.label), ask.then)));
    }

    /// Back: the overview, at the rival the sheet showed.
    fn back(&mut self, cx: &mut Cx) {
        self.maps.release(cx.assets);
        reputation_overview::open(cx, self.bin, self.args);
    }

    /// The question was answered No or the dialog was left with Back: the maps were let go, so read them again.
    fn start_maps(&mut self) {
        self.maps.start(self.page.map_textures());
    }
}

/// The page of `kind` for the rival `bin`, read from the career.
fn open_page(kind: Kind, bin: u8) -> Box<dyn Page> {
    match kind {
        Kind::Race => Box::new(race::RacePage::read(bin)),
        Kind::Milestones => Box::new(milestones::MilestonePage::read(bin)),
        Kind::Bounty => Box::new(bounty::BountyPage::read()),
    }
}

impl ScreenLogic for Sheet {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.screen_list_look(ListScreen::RepSheets);
        self.start_maps();
        self.refresh(cx);
    }

    fn resume(&mut self, cx: &mut Cx) {
        // A question that was answered No (or left) comes back here: the pictures were let go for the leave, so read them again.
        self.start_maps();
        self.page = open_page(self.kind, self.bin);
        self.at = self.at.min(self.len().saturating_sub(1));
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.vertical = cx.screen_list_look(ListScreen::RepSheets);
        self.maps.poll(cx.assets);
        self.zoom.tick(dt);
        self.place_map(cx);
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let entries = (0..self.len())
            .map(|at| MenuEntry {
                label: self.page.name(at),
                enabled: self.page.status(at) != Status::Locked,
                value: MenuValue::Choice { text: self.page.summary(at), arrows: false },
                icon: None,
            })
            .collect();
        Some(MenuList {
            title: Text::Game(self.page.heading()).show(),
            message: String::new(),
            entries,
            selected: self.at,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Sub,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if row >= self.len() {
            return;
        }
        self.at = row;
        self.refresh(cx);
        if matches!(pick, Pick::Press(..)) {
            self.accept(cx);
        }
    }

    // The pad's accept and back are not answered by the packages with a game message, so they come as pad bits.
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        if bits & pad::ACCEPT != 0 {
            self.accept(cx);
        }
        if bits & pad::BACK != 0 {
            self.back(cx);
        }
    }

    // The mouse takes the two scroll arrows at the left of the panel.
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if reputation_overview::hit(cx, ARROW_UP, point) {
            self.step(cx, -1);
            return true;
        }
        if reputation_overview::hit(cx, ARROW_DOWN, point) {
            self.step(cx, 1);
            return true;
        }
        false
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.step(cx, -(steps.signum() as isize));
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT | PAD_UP => self.step(cx, -1),
            PAD_RIGHT | PAD_DOWN => self.step(cx, 1),
            PAD_BACK => {}
            _ => {}
        }
    }

    fn footer(&self) -> Footer {
        match self.len() > 0 && self.page.ask(self.at).is_some() {
            true => Footer::BackAccept,
            false => Footer::BackOnly,
        }
    }
}
