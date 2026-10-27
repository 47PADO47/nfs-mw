//! The licence plate of a mod car: typing the text it wears (docs/modding/car-customization.md, "License plates").
//!
//! The page is the game's notice dialog with a keyboard prompt, the way an alias is named (`frontend/name_entry.rs`), reached
//! from the Visual category of a car whose manifest has a `[customization.plate]`. Accept keeps the text for the car beside
//! the profile's save (`garage/plates.rs`) and shows it on the car in the garage; an empty text goes back to the car's own.
//! It costs nothing and is not part of the cart: it is the player's registration, which the 2012 game keeps for the player too.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};
use nfsmw_data::car::mods::custom::{self, Plate};

use super::{GarageUi, Request};
use crate::frontend::logic::{Cx, Footer, Pick, ScreenLogic};
use crate::frontend::name_entry::TextKey;
use crate::frontend::{confirm, ids};
use crate::garage::{MenuEvent, plates};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

const HEADER: &str = "License Plate";
const PROMPT: &str = "Enter plate text:";

pub(super) struct PlateEntry {
    handle: GarageUi,
    folder: String,
    /// The plate of the car in the garage; `None` for a car without one (the page is then not offered).
    plate: Option<Plate>,
    text: String,
    vertical: bool,
    done: bool,
}

impl PlateEntry {
    /// The prompt starts with the text the car wears now: the one the player kept, else the car's own.
    pub fn new(handle: GarageUi) -> Self {
        let folder = handle.model().draft.car.clone();
        let plate = custom::of(&folder).and_then(|own| own.plate.clone());
        let text = plate.as_ref().map(|plate| worn(plate, &folder)).unwrap_or_default();
        Self { handle, folder, plate, text, vertical: false, done: false }
    }

    /// The text the notice shows: the prompt, the typed text and a caret.
    fn shown(&self) -> String {
        format!("{PROMPT} {}_", self.text)
    }

    fn refresh(&self, cx: &mut Cx) {
        confirm::set_text(cx, confirm::MESSAGE, &Text::Literal(self.shown()));
    }

    /// Keeps the text, shows it on the car and leaves; a text of spaces only is no text, and the car wears its own again.
    fn accept(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        let Some(plate) = &self.plate else {
            self.handle.request(Request::Pop);
            return;
        };
        let text = plate.registration(&self.text);
        let chosen = (!text.trim().is_empty()).then_some(text);
        if let Err(error) = plates::set(&self.folder, chosen.as_deref()) {
            log::warn!("{}: the plate text is not kept: {error:#}", self.folder);
        }
        // The garage builds the car from the draft: it carries the text, so the car is drawn again with it. The installed
        // setup and the cart carry it too, as nothing was bought: leaving the garage has no change to discard.
        let mut state = self.handle.0.lock().unwrap();
        state.model.draft.parts.plate = chosen.clone();
        state.baseline.parts.plate = chosen.clone();
        state.cart.parts.plate = chosen;
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(MenuEvent::Preview(selection)));
        state.requests.push(Request::Pop);
    }
}

/// The text the car in `folder` wears with no text typed in the garage: the player's own, else the manifest's, as typed.
fn worn(plate: &Plate, folder: &str) -> String {
    let kept = plates::text_for(folder);
    plate.worn(kept.as_deref()).map(|text| plate.registration(text)).unwrap_or_default()
}

impl ScreenLogic for PlateEntry {
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

    fn wants_text(&self) -> bool {
        true
    }

    fn typed(&mut self, cx: &mut Cx, key: TextKey) {
        let Some(plate) = &self.plate else { return };
        match key {
            TextKey::Backspace => {
                self.text.pop();
            }
            TextKey::Char(c) => {
                // Only what the plate's sheet can draw, upper case, and no more than the plate shows.
                if let Some(glyph) = plate.glyph(c).filter(|_| self.text.chars().count() < plate.slots as usize) {
                    self.text.push(glyph);
                }
            }
        }
        self.refresh(cx);
    }

    fn click(&mut self, _cx: &mut Cx, _point: glam::Vec2) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, _cx: &mut Cx, _row: usize, pick: Pick) {
        if matches!(pick, Pick::Press(..)) {
            self.accept();
        }
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let resolve = |text: &Text| text.resolve().unwrap_or_default();
        Some(MenuList {
            title: HEADER.to_owned(),
            message: self.shown(),
            entries: vec![MenuEntry::plain(resolve(&Text::Game(confirm::CONTINUE)), true)],
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Dialog,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn message(&mut self, _cx: &mut Cx, message: u32) {
        match message {
            confirm::ANSWERED | PAD_ACCEPT => self.accept(),
            PAD_BACK => {
                self.done = true;
                self.handle.request(Request::Pop);
            }
            _ => {}
        }
    }

    fn footer(&self) -> Footer {
        Footer::BackAccept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate() -> Plate {
        Plate {
            text: Some("NEED4SPD".into()),
            picture: "CAR_PLATE".into(),
            glyphs: "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789".chars().collect(),
            columns: 9,
            rows: 4,
            slots: 8,
            aliases: vec![('0', 'O')],
            ..Default::default()
        }
    }

    #[test]
    fn the_prompt_shows_the_text_and_a_caret() {
        let entry = PlateEntry {
            handle: GarageUi::default(),
            folder: "CAR".into(),
            plate: Some(plate()),
            text: "VELOCITY".into(),
            vertical: false,
            done: false,
        };
        assert_eq!(entry.shown(), "Enter plate text: VELOCITY_");
    }

    #[test]
    fn the_player_text_is_kept_in_the_registration_the_plate_draws() {
        let own = plate();
        assert_eq!(worn(&own, "NO_SUCH_CAR_ANYWHERE"), "NEED4SPD", "no kept text: the manifest's");
        let blank = Plate { text: None, ..own };
        assert_eq!(worn(&blank, "NO_SUCH_CAR_ANYWHERE"), "", "no text at all: the prompt starts empty");
    }
}
