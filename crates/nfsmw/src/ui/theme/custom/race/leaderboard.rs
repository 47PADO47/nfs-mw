//! The compact leaderboard (`docs/specs/custom-hud/race.md` section 4), slot `Leaderboard`, top-right. Hidden in
//! Minimal, where the strip's position is enough. In Balanced three rows: the racer ahead, the player, the racer behind
//! (two when the player leads or is last). In Advanced every row the race's board has (`hud/leaders.rs` keeps the four
//! best placed, the player's own place always among them). Each row: the place, the name, and a state word for a racer
//! out of the race, or the gap on the rival's row where the strip shows one.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::super::feed::{CustomFeed, CustomFeedCell};
use super::super::layout::{Anchor, HudBox};
use super::super::memory::Frame;
use super::super::mode::{Detail, HudMode};
use super::super::parts::{Drawn, Parts, Row, left, right};
use super::super::plate::PlateStyle;
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys};
use super::gap::gap_words;
use super::strip::StripModel;
use crate::hud::{HudState, LABEL_BUSTED, LABEL_KNOCKED_OUT, Leaders, RivalGap, game_word};

/// A row's height and the board's width at scale 1.
pub const ROW_HEIGHT: f32 = 16.0;
pub const WIDTH: f32 = 176.0;
const PLACE_W: f32 = 22.0;
const NAME_W: f32 = 82.0;
const STATUS_W: f32 = 64.0;

/// One row of the board.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardRow {
    pub place: u32,
    pub name: String,
    pub player: bool,
    /// `K.O.` or `BUSTED` for a racer out of the race, else the gap on the rival's row; `None` for the others.
    pub status: Option<String>,
}

/// The rows the board shows for `leaders`, the player in `place`, at `detail`. `gap` is the strip's rival gap, put on
/// the rival's row.
pub fn board_rows(leaders: &Leaders, place: u32, detail: Detail, gap: Option<&RivalGap>) -> Vec<BoardRow> {
    let shown = leaders.shown_rows();
    let window = match detail {
        Detail::Minimal => 0..0,
        Detail::Advanced => 0..shown,
        // The racer ahead, the player and the racer behind; the first three when the board has no row for the player.
        Detail::Balanced => match leaders.player {
            Some(at) => at.saturating_sub(1)..(at + 2).min(shown),
            None => 0..shown.min(3),
        },
    };
    let row = |i: usize| {
        let racer = &leaders.rows[i];
        let player = leaders.player == Some(i);
        let status = match (racer.busted, racer.out) {
            (true, _) => Some(game_word(LABEL_BUSTED, "BUSTED")),
            (false, true) => Some(game_word(LABEL_KNOCKED_OUT, "K.O.")),
            (false, false) => gap.filter(|gap| !player && gap.name == racer.name).map(gap_words),
        };
        // The rows are the best placed in order; only the player's can be from further down.
        let at = match player {
            true => place,
            false => i as u32 + 1,
        };
        BoardRow { place: at, name: racer.name.clone(), player, status }
    };
    window.filter(|&i| i < leaders.rows.len()).map(row).collect()
}

/// The board's rows of the frame `hud` (none outside a race or with `race_info` off).
pub fn rows_of(hud: &HudState, detail: Detail, race_info: bool) -> Vec<BoardRow> {
    let (Some(leaders), Some(race)) = (hud.leaders.as_ref(), hud.race) else { return Vec::new() };
    if !race_info {
        return Vec::new();
    }
    let gap = StripModel::of(hud, detail).and_then(|strip| strip.rival);
    board_rows(leaders, race.place, detail, gap.as_ref())
}

/// The Custom `Leaderboard` component.
pub struct Leaderboard {
    feed: CustomFeedCell,
    drawn: Drawn,
}

impl Leaderboard {
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, drawn: Drawn::default() }
    }
}

impl Component for Leaderboard {
    /// The board places itself in the HUD's frame, not in `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let frame = self.feed.memory().frame(&feed);
        self.drawn = draw(&feed, &frame, cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.drawn.build(out);
    }
}

/// The board of the frame, in `Racing`.
fn draw(feed: &CustomFeed, frame: &Frame, cx: &LayoutCx<'_>) -> Drawn {
    let fade = frame.opacity(|mode| mode == HudMode::Racing, &Motion::for_settings(&feed.settings));
    if fade <= 0.0 {
        return Drawn::default();
    }
    let rows = rows_of(&feed.hud, Detail::of(&feed.settings), feed.settings.race_info);
    if rows.is_empty() {
        return Drawn::default();
    }
    let mut parts = Parts::new(cx, HudBox::new(feed.screen, feed.settings.scale), &feed.settings);
    parts.fade(fade);
    for (i, board_row) in rows.iter().enumerate() {
        let rect = parts.place(Anchor::TopRight, (0.0, i as f32 * (ROW_HEIGHT + 1.0)), (WIDTH, ROW_HEIGHT));
        draw_row(&mut parts, rect, board_row);
    }
    parts.finish()
}

/// One row in `rect`: the player's on a raised plate with the accent line.
fn draw_row(parts: &mut Parts<'_>, rect: Rect, board_row: &BoardRow) {
    let (plate, text) = match board_row.player {
        true => (PlateStyle::RAISED.with_accent(keys::ACCENT), keys::TEXT),
        false => (PlateStyle::SURFACE, keys::TEXT_SECONDARY),
    };
    parts.plate(rect, plate);
    let mut row = Row::new(rect.min_x(), rect.min_y(), rect.h, parts.scale());
    row.skip(4.0);
    let place = Label::numeric(board_row.place.to_string(), size::NOTIFICATION_LINE).justify(left());
    parts.text(place, row.cell(PLACE_W));
    parts.text(
        Label::label(board_row.name.clone(), size::NOTIFICATION_LINE).colour(text).justify(left()),
        row.cell(NAME_W),
    );
    if let Some(status) = &board_row.status {
        let label = Label::numeric(status.clone(), size::HEADING).colour(keys::TEXT_SECONDARY).justify(right());
        parts.text(label, row.cell(STATUS_W));
    }
}
