//! The pursuit panel (`docs/specs/custom-hud/pursuit.md` section 2), slot `PursuitMeter`, top-right and compact:
//!
//! ```text
//!  [] ACTIVE              HEAT 3 ▮▮▮▯▯
//!  02:14                BOUNTY 12,500
//! ```
//!
//! The status is a word and an icon (a solid square for Active, a hollow one for Search, a filling one for Cooldown),
//! so it never rests on colour alone. Per phase: Active in `danger`; Search turns the decoration down to the secondary
//! text and says when the police last saw the player; Cooldown in `nav` with its bar, which shows only in that phase.
//! A high heat steps up the plate's contrast and the number's size, and a heat level up holds a highlight for a pulse:
//! never a flash or a red overlay.
//!
//! In a race the race layout keeps the screen, and the chase is a compact strip (the status row only) stacked under the
//! leaderboard. At Escaped and Busted the panel leaves; the banner shows the end card (`end_card.rs`).

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};
use glam::Vec2;

use super::super::feed::{CustomFeed, CustomFeedCell};
use super::super::layout::{Anchor, HudBox};
use super::super::memory::Frame;
use super::super::mode::{Detail, HudMode};
use super::super::parts::{Drawn, Parts, Row, edges, left, right};
use super::super::plate::PlateStyle;
use super::super::race::leaderboard::{ROW_HEIGHT, rows_of};
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys, ramp};
use super::super::words::{clock, grouped};
use super::model::{HEAT_SEGMENTS, PanelModel, Status};
use crate::i18n::t;

/// The panel's width, its rows' heights and padding, at scale 1.
const WIDTH: f32 = 186.0;
const STATUS_ROW: f32 = 22.0;
const LINE: f32 = 15.0;
const BAR_ROW: f32 = 10.0;
const PAD: f32 = 7.0;
const ICON: f32 = 7.0;
/// The heat number's growth at a high heat.
const HIGH_HEAT_GROWTH: f32 = 1.15;

/// The Custom `PursuitMeter` component.
pub struct PursuitPanel {
    feed: CustomFeedCell,
    drawn: Drawn,
}

impl PursuitPanel {
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, drawn: Drawn::default() }
    }
}

impl Component for PursuitPanel {
    /// The panel places itself in the HUD's frame, not in `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let frame = self.feed.memory().frame(&feed);
        self.drawn = draw(&feed, &frame, cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.drawn.build(out);
    }
}

/// Whether the panel shows in `mode`: a running chase outside a race, or any chase in a race (as the compact strip).
fn shows(mode: HudMode) -> bool {
    matches!(mode, HudMode::Racing) || mode.pursuit_phase().and_then(Status::of).is_some()
}

/// The panel of the frame; nothing when no chase runs.
fn draw(feed: &CustomFeed, frame: &Frame, cx: &LayoutCx<'_>) -> Drawn {
    let motion = Motion::for_settings(&feed.settings);
    let detail = Detail::of(&feed.settings);
    let Some(model) = PanelModel::of(&feed.hud, detail, feed.settings.pursuit_info) else { return Drawn::default() };
    let fade = frame.opacity(shows, &motion);
    if fade <= 0.0 {
        return Drawn::default();
    }
    let mut parts = Parts::new(cx, HudBox::new(feed.screen, feed.settings.scale), &feed.settings);
    parts.fade(fade);
    let in_race = frame.mode() == HudMode::Racing;
    // In a race the chase stacks under the leaderboard.
    let below = match in_race {
        true => rows_of(&feed.hud, detail, feed.settings.race_info).len() as f32 * (ROW_HEIGHT + 1.0) + 4.0,
        false => 0.0,
    };
    let rows = match in_race {
        true => Vec::new(),
        false => detail_rows(&model),
    };
    let bars = bar_rows(&model);
    let height = STATUS_ROW + LINE * rows.len() as f32 + BAR_ROW * bars.len() as f32 + 2.0;
    let rect = parts.place(Anchor::TopRight, (0.0, below), (WIDTH, height));
    let style = match (model.status, model.high_heat()) {
        (Status::Active, true) => PlateStyle::RAISED.with_accent(keys::DANGER),
        (Status::Active, false) => PlateStyle::SURFACE.with_accent(keys::DANGER),
        (Status::Search, _) => PlateStyle::SURFACE,
        (Status::Cooldown, _) => PlateStyle::SURFACE.with_accent(keys::NAV),
    };
    parts.plate(rect, style);
    let s = parts.scale();
    let status_rect = edges(rect.min_x(), rect.min_y(), rect.max_x(), rect.min_y() + STATUS_ROW * s);
    status_row(&mut parts, status_rect, &model, frame, &motion);
    let mut top = status_rect.max_y();
    for (left_text, right_text) in rows {
        let line = edges(rect.min_x() + PAD * s, top, rect.max_x() - PAD * s, top + LINE * s);
        parts.text(
            Label::numeric(left_text, size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY).justify(left()),
            line,
        );
        parts.text(Label::numeric(right_text, size::NOTIFICATION_LINE).justify(right()), line);
        top += LINE * s;
    }
    for (label, fraction, key) in bars {
        let line = edges(rect.min_x() + PAD * s, top, rect.max_x() - PAD * s, top + BAR_ROW * s);
        bar_row(&mut parts, line, &label, fraction, key);
        top += BAR_ROW * s;
    }
    parts.finish()
}

/// The status icon and word at the left, the heat at the right.
fn status_row(parts: &mut Parts<'_>, rect: Rect, model: &PanelModel, frame: &Frame, motion: &Motion) {
    let (key, word) = match model.status {
        Status::Active => (keys::DANGER, t!("hud-pursuit-status-active")),
        Status::Search => (keys::TEXT_SECONDARY, t!("hud-pursuit-status-search")),
        Status::Cooldown => (keys::NAV, t!("hud-pursuit-status-cooldown")),
    };
    let s = parts.scale();
    let mut row = Row::new(rect.min_x(), rect.min_y(), rect.h, s);
    row.skip(PAD);
    let icon_cell = row.cell(ICON + 4.0);
    status_icon(parts, Rect::new(icon_cell.x - 2.0 * s, icon_cell.y, ICON * s, ICON * s), model.status, key);
    let word_rect = row.cell(72.0);
    parts.text(Label::heading(word.to_uppercase(), size::HEADING).colour(key).justify(left()), word_rect);
    let Some(heat) = model.heat else { return };
    // The heat at the right: the heading, the number (larger at a high heat, highlighted for a pulse when it rises),
    // and the five segments.
    let bar = Rect::new(rect.max_x() - (PAD + 22.0) * s, rect.y, 44.0 * s, 6.0 * s);
    let segment_key = match model.status {
        Status::Search => keys::TEXT_SECONDARY,
        Status::Active | Status::Cooldown => keys::DANGER,
    };
    parts.segments(bar, HEAT_SEGMENTS, (heat as usize).min(HEAT_SEGMENTS), segment_key);
    let rose = frame.marks.heat.filter(|mark| mark.previous.is_some_and(|before| before < mark.value));
    let pulsing = rose.is_some_and(|mark| ramp(frame.age(mark.since), motion.pulse) < 1.0);
    let colour = match pulsing {
        true => keys::ACCENT,
        false => keys::TEXT,
    };
    let grow = match model.high_heat() {
        true => HIGH_HEAT_GROWTH,
        false => 1.0,
    };
    let number = Rect::new(bar.min_x() - 9.0 * s, rect.y, 14.0 * s, rect.h);
    parts.text(Label::numeric(heat.to_string(), size::STRIP_VALUE * grow).colour(colour).justify(right()), number);
    let heading = Rect::new(number.min_x() - 18.0 * s, rect.y, 30.0 * s, rect.h);
    parts.text(Label::heading(t!("hud-pursuit-heat").to_uppercase(), size::HEADING).justify(right()), heading);
}

/// The status icon in `rect`: a solid square, a hollow one, or a hollow one filling from the bottom.
fn status_icon(parts: &mut Parts<'_>, rect: Rect, status: Status, key: &'static str) {
    let outline = vec![
        Vec2::new(rect.min_x(), rect.min_y()),
        Vec2::new(rect.max_x(), rect.min_y()),
        Vec2::new(rect.max_x(), rect.max_y()),
        Vec2::new(rect.min_x(), rect.max_y()),
        Vec2::new(rect.min_x(), rect.min_y()),
    ];
    match status {
        Status::Active => parts.fill(key, rect),
        Status::Search => parts.stroke(outline, 1.0, key),
        Status::Cooldown => {
            parts.stroke(outline, 1.0, key);
            parts.fill(key, edges(rect.min_x(), rect.y, rect.max_x(), rect.max_y()));
        }
    }
}

/// The text rows under the status: the duration and the bounty, the last sighting in Search, and in Advanced the cost to
/// the state and the units. Each row is a left and a right text; a row with nothing to say is left out.
fn detail_rows(model: &PanelModel) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if model.duration.is_some() || model.bounty.is_some() {
        let duration = model.duration.map(clock).unwrap_or_default();
        let bounty =
            model.bounty.map(|bounty| t!("hud-pursuit-bounty-earned", bounty = grouped(bounty))).unwrap_or_default();
        rows.push((duration, bounty));
    }
    if let Some(age) = model.last_seen {
        rows.push((t!("hud-pursuit-last-seen", secs = age.round() as u32), String::new()));
    }
    if let Some(cost) = model.cost_to_state {
        let units = model.units.map(|(engaged, wrecked)| t!("hud-pursuit-units", engaged = engaged, wrecked = wrecked));
        rows.push((t!("hud-pursuit-cost", cost = grouped(cost)), units.unwrap_or_default()));
    }
    rows
}

/// The bars under the rows: the cooldown meter, the hiding latch, the bust meter, each with its label and colour.
fn bar_rows(model: &PanelModel) -> Vec<(String, f32, &'static str)> {
    let mut bars = Vec::new();
    if let Some(cooldown) = model.cooldown {
        bars.push((t!("hud-pursuit-status-cooldown"), cooldown, keys::NAV));
    }
    if let Some(hiding) = model.hiding {
        bars.push((t!("hud-pursuit-hiding"), hiding, keys::SUCCESS));
    }
    if let Some(bust) = model.bust {
        bars.push((t!("hud-pursuit-busted"), bust, keys::DANGER));
    }
    bars
}

/// One labelled bar in `rect`: the label at the left, the bar after it.
fn bar_row(parts: &mut Parts<'_>, rect: Rect, label: &str, fraction: f32, key: &'static str) {
    let s = parts.scale();
    let words = edges(rect.min_x(), rect.min_y(), rect.min_x() + 64.0 * s, rect.max_y());
    parts.text(Label::heading(label.to_uppercase(), size::HEADING).justify(left()), words);
    let bar = edges(words.max_x(), rect.y - 2.0 * s, rect.max_x(), rect.y + 2.0 * s);
    parts.bar(bar, fraction, key);
}
