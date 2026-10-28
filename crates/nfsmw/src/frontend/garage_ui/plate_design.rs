//! The design of a mod car's licence plate: the background behind the glyphs (docs/modding/car-customization.md, "Plate
//! designs").
//!
//! The page is the game's notice dialog, the way the plate text is entered (`plate.rs`), reached from the Visual category of a
//! car whose plate offers designs. Left and Right step through the designs (L and R ten at a time) and the car in the garage
//! shows each as it is reached; Accept keeps the design for the car beside the profile's save (`garage/plates.rs`), Back puts
//! the one the car wore before. In a theme that lays menus out as vertical lists the dialog shows a window of the names
//! around the one picked. Like the text, a design costs nothing and is not part of the cart.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_LTRIGGER, PAD_RIGHT, PAD_RTRIGGER, PAD_UP};
use nfsmw_data::car::customization::PlateDesignChoice;
use nfsmw_data::car::mods::custom;

use super::{GarageUi, Request};
use crate::frontend::logic::{Cx, Footer, Pick, ScreenLogic};
use crate::frontend::{confirm, ids};
use crate::garage::{MenuEvent, plates};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

const HEADER: &str = "Plate Design";
/// The first entry: no design, the plate as its sheet draws it.
const PLAIN: &str = "Plain plate";
/// How far L and R move through the list, and how many names the vertical list shows at once.
const JUMP: isize = 10;
const WINDOW: usize = 9;

/// One entry of the list: the id kept for it (empty for the plain plate) and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    id: String,
    label: String,
}

/// The entries for the designs a car offers: the plain plate first, then each design in the order of the index.
fn entries(designs: &[PlateDesignChoice]) -> Vec<Entry> {
    let plain = Entry { id: String::new(), label: PLAIN.to_owned() };
    let listed = designs.iter().map(|design| Entry { id: design.id.clone(), label: design.name.clone() });
    std::iter::once(plain).chain(listed).collect()
}

/// The position of the design `id` in `entries`; the plain plate (the first) for an id the list lacks.
fn position(entries: &[Entry], id: Option<&str>) -> usize {
    id.and_then(|id| entries.iter().position(|entry| entry.id == id)).unwrap_or(0)
}

/// `at` moved `by` entries along a list of `len`, wrapping round at either end.
fn stepped(at: usize, by: isize, len: usize) -> usize {
    match len {
        0 => 0,
        _ => (at as isize + by).rem_euclid(len as isize) as usize,
    }
}

/// The first entry of the window of `WINDOW` names that keeps `at` near its middle.
fn window_start(at: usize, len: usize) -> usize {
    at.saturating_sub(WINDOW / 2).min(len.saturating_sub(WINDOW))
}

pub(super) struct PlateDesigns {
    handle: GarageUi,
    folder: String,
    entries: Vec<Entry>,
    /// The entry the cursor is on.
    at: usize,
    /// What the draft had when the page opened, which Back puts back.
    before: Option<String>,
    vertical: bool,
    done: bool,
}

impl PlateDesigns {
    /// The cursor starts on the design the car wears now: the one picked in this visit, else the one the player kept, else the
    /// manifest's default, else the plain plate.
    pub fn new(handle: GarageUi) -> Self {
        let model = handle.model();
        let folder = model.draft.car.clone();
        let designs = model
            .selected()
            .and_then(|car| car.parts.as_ref())
            .map(|catalog| catalog.plate_designs.clone())
            .unwrap_or_default();
        let entries = entries(&designs);
        let before = model.draft.parts.plate_design.clone();
        let default =
            custom::of(&folder).and_then(|own| own.plate.as_ref()).and_then(|plate| plate.default_design.clone());
        let worn = before.clone().or_else(|| plates::design_for(&folder)).or(default);
        let at = position(&entries, worn.as_deref());
        Self { handle, folder, entries, at, before, vertical: false, done: false }
    }

    /// The text the notice shows: which design, of how many.
    fn shown(&self) -> String {
        let entry = &self.entries[self.at];
        format!("Design {} of {}: {}", self.at + 1, self.entries.len(), entry.label)
    }

    fn refresh(&self, cx: &mut Cx) {
        confirm::set_text(cx, confirm::MESSAGE, &Text::Literal(self.shown()));
    }

    /// Moves the cursor and shows the car with the design under it.
    fn go(&mut self, to: usize) {
        self.at = to.min(self.entries.len().saturating_sub(1));
        let id = self.entries[self.at].id.clone();
        let mut state = self.handle.0.lock().unwrap();
        state.model.draft.parts.plate_design = Some(id);
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(MenuEvent::Preview(selection)));
    }

    fn step(&mut self, by: isize) {
        let to = stepped(self.at, by, self.entries.len());
        self.go(to);
    }

    /// Keeps the design for the car, shows it and leaves.
    fn accept(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        let id = self.entries[self.at].id.clone();
        if let Err(error) = plates::set_design(&self.folder, Some(&id)) {
            log::warn!("{}: the plate design is not kept: {error:#}", self.folder);
        }
        // As with the text, nothing was bought: the installed setup and the cart carry the design too, so leaving the garage
        // has no change to discard.
        let mut state = self.handle.0.lock().unwrap();
        state.model.draft.parts.plate_design = Some(id.clone());
        state.baseline.parts.plate_design = Some(id.clone());
        state.cart.parts.plate_design = Some(id);
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(MenuEvent::Preview(selection)));
        state.requests.push(Request::Pop);
    }

    /// Puts back what the car wore when the page opened and leaves.
    fn cancel(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        let mut state = self.handle.0.lock().unwrap();
        state.model.draft.parts.plate_design = self.before.clone();
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(MenuEvent::Preview(selection)));
        state.requests.push(Request::Pop);
    }
}

impl ScreenLogic for PlateDesigns {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.list_look();
        confirm::prepare_notice(cx);
        confirm::set_text(cx, ids::HEADER_TEXT, &Text::Literal(HEADER.to_owned()));
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.list_look();
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    /// In the vertical list the names are one above the other, so Up and Down are the package's Left and Right.
    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn click(&mut self, _cx: &mut Cx, _point: glam::Vec2) -> bool {
        self.vertical
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.step(-(steps as isize));
        self.refresh(cx);
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if !matches!(pick, Pick::Press(..)) {
            return;
        }
        let target = window_start(self.at, self.entries.len()) + row;
        if target == self.at {
            self.accept();
            return;
        }
        self.go(target);
        self.refresh(cx);
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let first = window_start(self.at, self.entries.len());
        let shown = &self.entries[first..(first + WINDOW).min(self.entries.len())];
        Some(MenuList {
            title: HEADER.to_owned(),
            message: self.shown(),
            entries: shown.iter().map(|entry| MenuEntry::plain(entry.label.clone(), true)).collect(),
            selected: self.at - first,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Dialog,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT | PAD_UP => self.step(-1),
            PAD_RIGHT | PAD_DOWN => self.step(1),
            PAD_LTRIGGER => self.step(-JUMP),
            PAD_RTRIGGER => self.step(JUMP),
            confirm::ANSWERED | PAD_ACCEPT => self.accept(),
            PAD_BACK => self.cancel(),
            _ => {}
        }
        self.refresh(cx);
    }

    fn footer(&self) -> Footer {
        Footer::BackAccept
    }
}

#[cfg(test)]
#[path = "plate_design_tests.rs"]
mod tests;
