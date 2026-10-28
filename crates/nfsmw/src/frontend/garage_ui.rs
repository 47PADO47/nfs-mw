//! The installed garage packages and their game-side controllers.
//! Spec: `docs/specs/garage-game-ui.md`. No developer widgets participate here.

mod carousel;
mod choices;
mod chrome;
mod details;
mod finance;
mod gauge_preview;
mod inspection;
mod numbers;
mod pages;
mod palette;
mod plate;
mod plate_design;
pub(super) mod prompts;
mod table;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};

use super::logic::ScreenLogic;
use crate::garage::menu_rows::Row;
use crate::garage::{MenuEvent, Model, Selection};

pub use pages::{Page, Section};

/// Commands cross the same scene transaction boundary as the original garage.
#[derive(Clone, Debug)]
pub enum Request {
    Push(Page),
    Replace(Page),
    Pop,
    Event(Box<MenuEvent>),
    Backroom(bool),
    View(crate::garage::View, &'static str),
    /// Purchase/save the cart, then keep the garage open on its root screen.
    Save,
    Main,
    SaveTuning {
        resume: bool,
    },
}

impl Request {
    fn event(event: MenuEvent) -> Self {
        Self::Event(Box::new(event))
    }
}

#[derive(Default)]
struct State {
    model: Model,
    baseline: Selection,
    cart: Selection,
    requests: Vec<Request>,
    initialized: bool,
    view: Option<crate::garage::View>,
    section: Option<Section>,
    failure: u64,
}

/// Screens share one session, but do not own persistence or career money.
#[derive(Clone, Default)]
pub struct GarageUi(Arc<Mutex<State>>);

impl GarageUi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sync(&self, model: &Model) {
        let mut state = self.0.lock().unwrap();
        if !state.initialized {
            state.baseline = model.draft.clone();
            state.cart = model.draft.clone();
            state.initialized = true;
        }
        state.model = model.clone();
    }

    pub fn saved(&self, model: &Model) {
        let mut state = self.0.lock().unwrap();
        state.baseline = model.draft.clone();
        state.cart = model.draft.clone();
        state.model = model.clone();
    }

    /// A failed scene transaction leaves the current native screen available for retry.
    pub fn failed(&self, model: &Model) {
        let mut state = self.0.lock().unwrap();
        state.model = model.clone();
        state.failure = state.failure.wrapping_add(1);
    }

    fn failure(&self) -> u64 {
        self.0.lock().unwrap().failure
    }

    fn recover(&self, cx: &mut super::logic::Cx, seen: &mut u64) -> bool {
        let failure = self.failure();
        if failure == *seen {
            return false;
        }
        *seen = failure;
        cx.rt.set_input_enabled(cx.package, true);
        cx.rt.post_to_package(cx.package, 0xbdfd_2df9);
        cx.rt.post_to_package(cx.package, super::ids::INPUT_ENABLE);
        true
    }

    pub fn drain(&self) -> Vec<Request> {
        std::mem::take(&mut self.0.lock().unwrap().requests)
    }

    fn request(&self, request: Request) {
        self.0.lock().unwrap().requests.push(request);
    }

    fn enter(&self, page: Page) {
        if matches!(
            page,
            Page::Delete
                | Page::Leave
                | Page::Checkout { .. }
                | Page::MaxPerformance
                | Page::TuningDiscard
                | Page::TuningError
                | Page::InsufficientFunds
        ) {
            return;
        }
        let mut state = self.0.lock().unwrap();
        if let Page::Categories { section, .. } = page {
            state.section = Some(section);
        }
        let view = match page {
            Page::Showcase => state.view.unwrap_or(page.view()),
            Page::Main if state.model.backroom => crate::garage::View::Backroom,
            _ => page.view(),
        };
        state.view = Some(view);
        let camera = match (page, view) {
            (Page::Main, crate::garage::View::Backroom) => "customize_customizemain_2049",
            _ => page.camera(),
        };
        state.requests.push(Request::View(view, camera));
    }

    fn model(&self) -> Model {
        self.0.lock().unwrap().model.clone()
    }

    fn header(&self, cx: &mut super::logic::Cx, page: Page) {
        let state = self.0.lock().unwrap();
        chrome::refresh(cx, page, &state.model);
        finance::refresh(cx, page, &state.model, &state.baseline, &state.cart, state.section);
    }

    fn preview(&self, row: Row, at: usize) {
        let mut state = self.0.lock().unwrap();
        row.set(&mut state.model, at);
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(MenuEvent::Preview(selection)));
    }

    fn restore(&self, selection: Selection) {
        self.0.lock().unwrap().model.draft = selection.clone();
        self.request(Request::event(MenuEvent::Preview(selection)));
    }

    fn add_to_cart(&self) {
        let mut state = self.0.lock().unwrap();
        state.cart = state.model.draft.clone();
    }

    fn show_cart(&self) {
        let state = self.0.lock().unwrap();
        let selection = state.cart.clone();
        let page = Page::Cart { backroom: state.model.backroom, quick: !state.model.shop };
        drop(state);
        self.restore(selection);
        self.request(Request::Push(page));
    }

    pub fn logic(&self, page: Page) -> Box<dyn ScreenLogic> {
        match page {
            Page::Numbers => Box::new(numbers::Numbers::new(self.clone())),
            Page::Plate => Box::new(plate::PlateEntry::new(self.clone())),
            Page::PlateDesign => Box::new(plate_design::PlateDesigns::new(self.clone())),
            Page::Paint { .. } | Page::GaugeColors { .. } => Box::new(palette::Palette::new(self.clone(), page)),
            Page::Tuning
            | Page::Cart { .. }
            | Page::Manager
            | Page::AddCar
            | Page::Delete
            | Page::Leave
            | Page::TuningDiscard
            | Page::TuningError
            | Page::InsufficientFunds
            | Page::Checkout { .. }
            | Page::MaxPerformance => table::logic(self.clone(), page),
            _ => Box::new(carousel::Carousel::new(self.clone(), page)),
        }
    }
}
