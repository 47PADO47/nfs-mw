//! A rival's picture and facts in the Remastered look: the card at the right of the Blacklist, and the bio page that shows one
//! rival in full. Both read a `MenuValue::Rival`.

use blackbox_ui::{Image, Outline, Rect};

use super::blacklist::{rival_at, status_colour, status_text};
use super::dialog::wrap_text;
use super::layout as place;
use super::{Parts, fitted};
use crate::ui::theme::remastered::skin::keys;
use crate::ui::theme::{MenuList, RivalInfo};

/// The picture's frame, the name's sizes and the rows of facts under it.
const FRAME_RADIUS: f32 = 3.0;
const FRAME_EDGE: f32 = 1.2;
const NAME_SCALE: f32 = 2.1;
const ALIAS_SCALE: f32 = 1.35;
const LABEL_SCALE: f32 = 1.2;
const VALUE_SCALE: f32 = 1.85;
const FACT_PITCH: f32 = 33.0;
const LOCKED_PICTURE: [u8; 4] = [70, 72, 80, 255];
/// The bio page: the picture's side and its place, the text column's gap to it, the paragraph's size and line distance.
const BIO_PICTURE: f32 = 200.0;
const BIO_LEFT: f32 = 44.0;
const BIO_TOP: f32 = -176.0;
const BIO_GAP: f32 = 34.0;
const BIO_MARGIN: f32 = 44.0;
const BIO_SCALE: f32 = 1.45;
const BIO_PITCH: f32 = 17.0;
const TITLE_Y: f32 = -208.0;

impl Parts<'_, '_> {
    /// A rival's card in `area`: the picture across its top, then the name, the alias, the bounty and the status.
    pub(super) fn rival_card(&mut self, name: &str, info: &RivalInfo, area: Rect) {
        let side = area.w.min(190.0);
        let picture = Rect::new(area.min_x() + side / 2.0, area.min_y() + side / 2.0, side, side);
        self.rival_picture(info, picture);
        let left = area.min_x();
        let mut y = picture.max_y() + 20.0;
        let name = name.to_uppercase();
        self.text(
            keys::FONT_MENU_TITLE,
            &name,
            keys::MENU_TEXT_SELECTED,
            fitted(&name, NAME_SCALE, area.w),
            (left, y),
            false,
        );
        if !info.alias.is_empty() {
            y += 19.0;
            self.text(keys::FONT_MENU, &info.alias.to_uppercase(), keys::MENU_TAGLINE, ALIAS_SCALE, (left, y), false);
        }
        y += 28.0;
        self.fact("BOUNTY", &format!("${}", info.bounty), keys::MENU_TEXT_SELECTED, (left, y));
        self.fact("STATUS", status_text(info.status), status_colour(info.status), (left, y + FACT_PITCH));
    }

    /// A rival's picture with a thin accent frame. A rival not reached yet is dimmed.
    fn rival_picture(&mut self, info: &RivalInfo, rect: Rect) {
        if info.portrait != 0 {
            let tint = if info.status == crate::ui::theme::RivalStatus::Locked { LOCKED_PICTURE } else { [255; 4] };
            self.push(Image::raw(info.portrait).with_tint(tint), rect);
        }
        let frame = Outline::new(keys::MENU_TITLE, rect, FRAME_RADIUS, FRAME_EDGE, self.cx);
        self.add(frame);
    }

    /// A small label with its value under it.
    fn fact(&mut self, label: &str, value: &str, colour: &str, at: (f32, f32)) {
        self.text(keys::FONT_MENU, label, keys::MENU_TAGLINE, LABEL_SCALE, at, false);
        self.text(keys::FONT_MENU, value, colour, VALUE_SCALE, (at.0, at.1 + 17.0), false);
    }

    /// The bio page: the title, the rival's picture on the left, and at its right the name, alias, car and strength, and the bio.
    pub(super) fn rival_bio(&mut self, list: &MenuList) {
        let left = place::left_edge(list.aspect);
        self.scrim(list.aspect);
        self.shade(left);
        let title = (left + place::TEXT_LEFT, TITLE_Y);
        self.text(
            keys::FONT_MENU_TITLE,
            &list.title.to_uppercase(),
            keys::MENU_TITLE,
            place::TITLE_SCALE,
            title,
            false,
        );
        if let Some((entry, info)) = rival_at(list, 0) {
            let side = BIO_PICTURE;
            let picture = Rect::new(left + BIO_LEFT + side / 2.0, BIO_TOP + side / 2.0, side, side);
            self.rival_picture(info, picture);
            let x = picture.max_x() + BIO_GAP;
            let room = place::right_edge(list.aspect) - BIO_MARGIN - x;
            self.rival_text(&entry.label, info, &list.message, x, room);
        }
        self.footer(&list.hints, left);
    }

    /// The text column of the bio page: names and facts, then the bio wrapped to `room` units.
    fn rival_text(&mut self, name: &str, info: &RivalInfo, bio: &str, x: f32, room: f32) {
        let name = name.to_uppercase();
        let mut y = BIO_TOP + 12.0;
        self.text(
            keys::FONT_MENU_TITLE,
            &name,
            keys::MENU_TEXT_SELECTED,
            fitted(&name, NAME_SCALE, room),
            (x, y),
            false,
        );
        for (label, value) in [("ALIAS", &info.alias), ("CAR", &info.car), ("STRENGTH", &info.strength)] {
            if value.is_empty() {
                continue;
            }
            y += 38.0;
            self.fact(label, &value.to_uppercase(), keys::MENU_TEXT, (x, y));
        }
        y += 48.0;
        for line in wrap_text(bio, room, BIO_SCALE) {
            self.text(keys::FONT_MENU, &line, keys::MENU_TEXT, BIO_SCALE, (x, y), false);
            y += BIO_PITCH;
        }
    }
}
