//! The Blacklist (`Blacklist`, `MenuStyle::Blacklist`): a surface scrim, the title and the count of rivals beaten at the
//! top, the fifteen rivals as rows (rank, picture, name, bounty and where the player stands with them), the selected row
//! on the raised plate with the accent bar, and the selected rival's card at the right when the window has room for it
//! (`rival.rs`). The rows' geometry is shared with the frontend's hit test ([`row_at`]).

use blackbox_ui::{Component, Image, Rect};
use glam::Vec2;

use super::layout::{BAR_WIDTH, MenuFrame};
use super::list::{DISABLED, fitted_size};
use super::{Align, Parts};
use crate::i18n::t;
use crate::ui::theme::custom::plate::PlateStyle;
use crate::ui::theme::custom::text::{Label, size};
use crate::ui::theme::custom::tokens::keys;
use crate::ui::theme::{MenuEntry, MenuList, MenuValue, RivalInfo, RivalStatus};

/// How many rivals the list holds.
pub const ROWS: usize = 15;
/// The scrim's alpha over the window.
pub(super) const SCRIM: f32 = 0.65;
/// The title's middle and the first row's top below the safe area's top, and the room kept above the footer line, at
/// scale 1.
const TITLE_Y: f32 = 20.0;
const TOP: f32 = 46.0;
const FOOTER_GAP: f32 = 18.0;
/// A row's pitch at text size 1, the gap between two rows' plates, and a row's width, at scale 1.
const ROW_PITCH: f32 = 24.0;
const ROW_GAP: f32 = 3.0;
const ROW_WIDTH: f32 = 430.0;
/// The rows' text size.
const TEXT_SIZE: f32 = 14.0;
const STATUS_SIZE: f32 = 12.0;
/// The columns, from the row's left edge at scale 1: the rank's right edge, the picture's left and side, the name's
/// left and room, the bounty's right edge, and the status's inset from the row's right.
const RANK_RIGHT: f32 = 34.0;
const PICTURE_LEFT: f32 = 42.0;
const PICTURE: f32 = 18.0;
const NAME_LEFT: f32 = 68.0;
const NAME_ROOM: f32 = 160.0;
const BOUNTY_RIGHT: f32 = 316.0;
const STATUS_INSET: f32 = 12.0;
/// The card's gap from the rows and the least width it needs, at scale 1.
const CARD_GAP: f32 = 24.0;
const CARD_LEAST: f32 = 140.0;
/// The tint of a picture of a rival not reached yet.
pub(super) const LOCKED_PICTURE: [u8; 4] = [70, 72, 80, 255];

/// Where the Blacklist's parts are, in one menu frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlacklistFrame {
    left: f32,
    width: f32,
    top: f32,
    bottom: f32,
    pitch: f32,
    title_y: f32,
    scale: f32,
    card: Option<Rect>,
}

impl BlacklistFrame {
    /// The Blacklist's places in `frame`.
    pub fn of(frame: &MenuFrame) -> Self {
        let s = frame.scale();
        let safe = frame.safe();
        let top = safe.min_y() + TOP * s;
        let bottom = frame.footer_y() - FOOTER_GAP * s;
        let pitch = frame.sizing().units(ROW_PITCH).min((bottom - top) / ROWS as f32);
        let left = frame.plate_left();
        let width = (ROW_WIDTH * s).min(safe.max_x() - left);
        let card_left = left + width + CARD_GAP * s;
        let card_width = safe.max_x() - card_left;
        let card = (card_width >= CARD_LEAST * s)
            .then(|| Rect::new(card_left + card_width / 2.0, (top + bottom) / 2.0, card_width, bottom - top));
        Self { left, width, top, bottom, pitch, title_y: safe.min_y() + TITLE_Y * s, scale: s, card }
    }

    /// The y of the middle of the `i`th row.
    pub fn row_y(&self, i: usize) -> f32 {
        self.top + self.pitch * (i as f32 + 0.5)
    }

    /// The plate of the row whose middle is at `y`.
    pub fn row_rect(&self, y: f32) -> Rect {
        Rect::new(self.left + self.width / 2.0, y, self.width, self.pitch - ROW_GAP * self.scale)
    }

    /// The left and right edges of the rows.
    pub fn left(&self) -> f32 {
        self.left
    }

    pub fn right(&self) -> f32 {
        self.left + self.width
    }

    /// The top of the first row and the lowest the last may reach (the bio page uses the same band).
    pub fn top(&self) -> f32 {
        self.top
    }

    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    /// The y of the title's middle.
    pub fn title_y(&self) -> f32 {
        self.title_y
    }

    /// The selected rival's card at the right of the rows, or `None` when the window is too narrow for it.
    pub fn card(&self) -> Option<Rect> {
        self.card
    }
}

/// The row of a Blacklist under a point of the screen, on a window `aspect` wide at `text_size`. The whole band of a
/// row answers, from the window's left edge to the end of its plate.
pub fn row_at(point: Vec2, list: &MenuList, aspect: f32, text_size: f32) -> Option<usize> {
    let rows = BlacklistFrame::of(&MenuFrame::new(aspect, text_size));
    if point.x > rows.right() {
        return None;
    }
    let shown = list.entries.len().min(ROWS);
    (0..shown).find(|&i| (point.y - rows.row_y(i)).abs() <= rows.pitch / 2.0)
}

/// The `i`th entry of a list with the rival it holds.
pub(super) fn rival_at(list: &MenuList, i: usize) -> Option<(&MenuEntry, &RivalInfo)> {
    let entry = list.entries.get(i)?;
    match &entry.value {
        MenuValue::Rival(info) => Some((entry, info)),
        _ => None,
    }
}

/// The colour key and alpha of a status: success for a rival beaten, the accent for the one in hand, faint for the rest.
pub(super) fn status_look(status: RivalStatus) -> (&'static str, f32) {
    match status {
        RivalStatus::Complete => (keys::SUCCESS, 1.0),
        RivalStatus::InProgress => (keys::ACCENT, 1.0),
        RivalStatus::Locked => (keys::TEXT_SECONDARY, DISABLED),
    }
}

/// A status as the screen writes it.
pub(super) fn status_text(status: RivalStatus) -> String {
    match status {
        RivalStatus::Complete => t!("race-rival-complete"),
        RivalStatus::InProgress => t!("race-rival-in-progress"),
        RivalStatus::Locked => t!("race-rival-locked"),
    }
}

impl Parts<'_, '_> {
    /// The Blacklist: the scrim, the title and count, a row per rival with the selection plate at `plate_y`, the
    /// selected rival's card and the footer.
    pub(super) fn blacklist(&mut self, list: &MenuList, plate_y: Option<f32>) {
        let frame = self.frame();
        let rows = BlacklistFrame::of(&frame);
        self.panel(keys::SURFACE, frame.window(), SCRIM);
        self.page_title(&list.title, rows.title_y());
        if !list.message.is_empty() {
            let count = Label::numeric(list.message.clone(), TEXT_SIZE).colour(keys::TEXT_SECONDARY);
            self.text(count, (rows.right() - STATUS_INSET * rows.scale, rows.title_y()), Align::Right);
        }
        if let Some(y) = plate_y {
            let rect = rows.row_rect(y);
            self.plate(rect, PlateStyle::RAISED);
            let bar = BAR_WIDTH * frame.scale();
            self.panel(keys::ACCENT, Rect::new(rect.min_x() + bar / 2.0, y, bar, rect.h), 1.0);
        }
        for (i, entry) in list.entries.iter().take(ROWS).enumerate() {
            self.rival_row(entry, i == list.selected, rows.row_y(i), &rows);
        }
        if let (Some(area), Some((entry, info))) = (rows.card(), rival_at(list, list.selected)) {
            self.rival_card(&entry.label, info, area);
        }
        self.footer(&list.hints);
    }

    /// A page's title in the spaced face at `y`, with the short accent rule under it.
    pub(super) fn page_title(&mut self, title: &str, y: f32) {
        if title.is_empty() {
            return;
        }
        let frame = self.frame();
        let s = frame.scale();
        let heading = Label::heading(title.to_uppercase(), size::MENU_TITLE).colour(keys::TEXT);
        self.text(heading, (frame.text_left(), y), Align::Left);
        let rule = Rect::new(frame.text_left() + 16.0 * s, y + frame.sizing().units(16.0), 32.0 * s, 2.0 * s);
        self.panel(keys::ACCENT, rule, 1.0);
    }

    /// One rival's row: rank, picture, name, bounty and status.
    fn rival_row(&mut self, entry: &MenuEntry, selected: bool, y: f32, rows: &BlacklistFrame) {
        let MenuValue::Rival(info) = &entry.value else { return };
        let s = rows.scale;
        let left = rows.left();
        let locked = info.status == RivalStatus::Locked;
        let (colour, alpha) = match (locked, selected) {
            (true, _) => (keys::TEXT_SECONDARY, DISABLED),
            (false, true) => (keys::TEXT, 1.0),
            (false, false) => (keys::TEXT_SECONDARY, 1.0),
        };
        let rank = Label::numeric(info.rank.to_string(), TEXT_SIZE).colour(colour).opacity(alpha);
        self.text(rank, (left + RANK_RIGHT * s, y), Align::Right);
        if info.portrait != 0 {
            let tint = if locked { LOCKED_PICTURE } else { [255; 4] };
            let rect = Rect::new(left + (PICTURE_LEFT + PICTURE / 2.0) * s, y, PICTURE * s, PICTURE * s);
            let mut picture = Image::raw(info.portrait).with_tint(tint);
            picture.layout(self.cx(), rect);
            self.add(picture);
        }
        let fit = fitted_size(&entry.label, TEXT_SIZE, NAME_ROOM * s, self.frame().sizing());
        let name = Label::label(entry.label.clone(), fit).colour(colour).opacity(alpha);
        self.text(name, (left + NAME_LEFT * s, y), Align::Left);
        let bounty = Label::numeric(info.bounty.clone(), TEXT_SIZE).colour(colour).opacity(alpha);
        self.text(bounty, (left + BOUNTY_RIGHT * s, y), Align::Right);
        let (status_colour, status_alpha) = status_look(info.status);
        let status = Label::label(status_text(info.status), STATUS_SIZE).colour(status_colour).opacity(status_alpha);
        self.text(status, (rows.right() - STATUS_INSET * s, y), Align::Right);
    }
}
