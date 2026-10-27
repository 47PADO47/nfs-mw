//! A rival's picture and facts in the Custom look: the card at the right of the Blacklist, and the bio page
//! (`MenuStyle::Rival`) that shows one rival in full. Both read a `MenuValue::Rival` and sit on Custom plates.

use blackbox_ui::{Component, Image, Outline, Rect};

use super::blacklist::{BlacklistFrame, SCRIM, picture_tint, rival_at, status_look, status_text};
use super::dialog::wrap;
use super::list::fitted_size;
use super::{Align, Parts};
use crate::hud::game_word;
use crate::i18n::t;
use crate::ui::theme::custom::plate::PlateStyle;
use crate::ui::theme::custom::text::{Label, size};
use crate::ui::theme::custom::tokens::keys;
use crate::ui::theme::{MenuList, RivalInfo, RivalStatus};

/// The language label of the Blacklist's bounty: `BL_BOUNTY`.
const LABEL_BOUNTY: u32 = 0x582C_4C0D;

/// The card: the room inside its plate, the largest its picture gets, and its text sizes, at scale 1.
const CARD_PAD: f32 = 12.0;
const CARD_PICTURE: f32 = 180.0;
const NAME_SIZE: f32 = 18.0;
const ALIAS_SIZE: f32 = 12.0;
const VALUE_SIZE: f32 = 15.0;
/// The distance from a fact's heading to its value, and between two facts, at text size 1.
const FACT_DROP: f32 = 15.0;
const FACT_PITCH: f32 = 36.0;
/// The bio page: the picture's side, the gap to the text column, the bio's line distance, at scale 1.
const BIO_PICTURE: f32 = 180.0;
const BIO_GAP: f32 = 24.0;
const BIO_PITCH: f32 = 18.0;
/// The bio's text size: the dialog message's, which `wrap` measures at.
const BIO_SIZE: f32 = 13.0;
/// The picture's frame.
const FRAME_RADIUS: f32 = 3.0;
const FRAME_EDGE: f32 = 1.0;

impl Parts<'_, '_> {
    /// A rival's card in `area`: a plate with the picture across its top, then the name, the alias, the bounty and the
    /// status.
    pub(super) fn rival_card(&mut self, name: &str, info: &RivalInfo, area: Rect) {
        let frame = self.frame();
        let s = frame.scale();
        let sizing = frame.sizing();
        self.plate(area, PlateStyle::SURFACE.with_accent(keys::ACCENT));
        let pad = CARD_PAD * s;
        let side = (CARD_PICTURE * s).min(area.w - 2.0 * pad).min(area.h / 2.0).max(0.0);
        let picture = Rect::new(area.min_x() + pad + side / 2.0, area.min_y() + pad + side / 2.0, side, side);
        self.rival_picture(info, picture);
        let left = area.min_x() + pad;
        let room = area.w - 2.0 * pad;
        let mut y = picture.max_y() + sizing.units(NAME_SIZE);
        let fit = fitted_size(name, NAME_SIZE, room, sizing);
        self.text(Label::label(name.to_owned(), fit).colour(keys::TEXT), (left, y), Align::Left);
        if !info.alias.is_empty() {
            y += sizing.units(ALIAS_SIZE + 6.0);
            let alias = Label::label(info.alias.clone(), ALIAS_SIZE).colour(keys::TEXT_SECONDARY);
            self.text(alias, (left, y), Align::Left);
        }
        y += sizing.units(FACT_PITCH * 0.75);
        let bounty = game_word(LABEL_BOUNTY, "Bounty");
        self.fact(&bounty, Label::numeric(info.bounty.clone(), VALUE_SIZE).colour(keys::TEXT), (left, y));
        let (colour, alpha) = status_look(info.status);
        let status = Label::label(status_text(info.status), VALUE_SIZE).colour(colour).opacity(alpha);
        self.fact(&t!("race-rival-status"), status, (left, y + sizing.units(FACT_PITCH)));
    }

    /// A rival's picture with a thin accent frame. A rival not reached yet is dimmed.
    fn rival_picture(&mut self, info: &RivalInfo, rect: Rect) {
        if info.portrait != 0 {
            let tint = picture_tint(self.cx().skin, info.status == RivalStatus::Locked);
            let mut picture = Image::raw(info.portrait).with_tint(tint);
            picture.layout(self.cx(), rect);
            self.add(picture);
        }
        let s = self.frame().scale();
        let frame = Outline::new(keys::ACCENT, rect, FRAME_RADIUS * s, FRAME_EDGE * s, self.cx());
        self.add(frame);
    }

    /// A short heading at `at`, with `value` under it.
    fn fact(&mut self, heading: &str, value: Label, at: (f32, f32)) {
        let drop = self.frame().sizing().units(FACT_DROP);
        self.text(Label::heading(heading.to_uppercase(), size::HEADING), at, Align::Left);
        self.text(value, (at.0, at.1 + drop), Align::Left);
    }

    /// The bio page: the title, the rival's picture on the left, and at its right the name, the alias, the car and the
    /// strength, then the bio wrapped to the room left, down to the footer.
    pub(super) fn rival_bio(&mut self, list: &MenuList) {
        let frame = self.frame();
        let rows = BlacklistFrame::of(&frame);
        self.panel(keys::SURFACE, frame.window(), SCRIM);
        self.page_title(&list.title, rows.title_y());
        if let Some((entry, info)) = rival_at(list, 0) {
            let s = frame.scale();
            let side = BIO_PICTURE * s;
            let picture = Rect::new(rows.left() + side / 2.0, rows.top() + side / 2.0, side, side);
            self.rival_picture(info, picture);
            let x = picture.max_x() + BIO_GAP * s;
            let room = (frame.safe().max_x() - x).max(0.0);
            self.rival_text(&entry.label, info, &list.message, (x, room), rows.bottom());
        }
        self.footer(&list.hints);
    }

    /// The bio page's text column at `x`, `room` wide: the name and the facts, then the bio, stopping at `bottom`.
    fn rival_text(&mut self, name: &str, info: &RivalInfo, bio: &str, (x, room): (f32, f32), bottom: f32) {
        let frame = self.frame();
        let sizing = frame.sizing();
        let mut y = BlacklistFrame::of(&frame).top() + sizing.units(NAME_SIZE) / 2.0;
        let fit = fitted_size(name, NAME_SIZE + 4.0, room, sizing);
        self.text(Label::label(name.to_owned(), fit).colour(keys::TEXT), (x, y), Align::Left);
        let facts = [
            (t!("race-rival-alias"), &info.alias),
            (t!("race-rival-car"), &info.car),
            (t!("race-rival-strength"), &info.strength),
        ];
        y += sizing.units(NAME_SIZE);
        for (heading, value) in facts {
            if value.is_empty() {
                continue;
            }
            let value = Label::label(value.clone(), VALUE_SIZE).colour(keys::TEXT);
            self.fact(&heading, value, (x, y));
            y += sizing.units(FACT_PITCH);
        }
        let pitch = sizing.units(BIO_PITCH);
        for line in wrap(bio, room, sizing) {
            if y > bottom {
                return;
            }
            self.text(Label::label(line, BIO_SIZE).colour(keys::TEXT_SECONDARY), (x, y), Align::Left);
            y += pitch;
        }
    }
}
