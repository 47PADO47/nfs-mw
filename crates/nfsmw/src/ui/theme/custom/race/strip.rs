//! The race status strip (`docs/specs/custom-hud/race.md` section 3), slot `RaceStatus`, top-centre: a compact strip,
//! not a leaderboard.
//!
//! ```text
//!  POS 2/6   LAP 2/3   ▮▮▮▮▮▮▮▯▯▯▯▯   4.2 km   VALDEZ ^ 1.42 s AHEAD
//! ```
//!
//! [`StripModel`] picks the fields from the frame (each present only where the race computes it); [`RaceStrip`] lays
//! them out. The strip arrives with a slide at `Racing`, pulses the position once when the place changes, and leaves
//! when the results flow takes the race. `[custom_hud] race_info` off takes it away.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};
use glam::Vec2;
use nfsmw_data::gameplay::RaceType;

use super::super::feed::{CustomFeed, CustomFeedCell};
use super::super::layout::{Anchor, HudBox};
use super::super::memory::Frame;
use super::super::mode::{Detail, HudMode};
use super::super::parts::{Drawn, Parts, Row, centred, left, right};
use super::super::plate::PlateStyle;
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys, ramp};
use super::super::words::{clock, distance_long};
use super::gap::{Side, gap_words};
use crate::hud::{HudState, RaceHud, RaceInfo, RivalGap, shows_rival_gap};
use crate::i18n::t;

/// The progress bar's segments.
pub const SEGMENTS: usize = 12;

/// The lap or the checkpoint the player is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// A lapped race: the lap and the laps; `last` on the final lap of a race of more than one.
    Lap { now: u32, of: u32, last: bool },
    /// A sprint or checkpoint race: the gate driven to and the gates in all.
    Gate { now: u32, of: u32 },
}

/// The figure a race ranks by, where it is not the place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Figure {
    /// A speed-trap race's points.
    Points(f32),
    /// A tollbooth race's time left.
    TimeLeft(f32),
}

/// The strip's fields for one frame. A field the race does not compute is `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct StripModel {
    pub place: u32,
    pub racers: u32,
    pub progress: Option<Progress>,
    /// How much of the race is done, 0 to 1; Balanced and Advanced.
    pub bar: Option<f32>,
    /// Metres of the route left; Balanced and Advanced.
    pub remaining_m: Option<f32>,
    /// The nearest rival and the gap; Balanced and Advanced, never in a race whose standing is not about time.
    pub rival: Option<RivalGap>,
    pub figure: Option<Figure>,
}

impl StripModel {
    /// The strip of `hud` at `detail`; `None` without a race readout.
    pub fn of(hud: &HudState, detail: Detail) -> Option<Self> {
        let race = hud.race?;
        let info = hud.race_info.as_ref();
        let kind = info.map(|info| info.kind);
        // A drag race shows the place and the distance only: the shift cue is its instrument.
        let drag = hud.drag || kind == Some(RaceType::Drag);
        let more = detail >= Detail::Balanced;
        let timed = kind.is_none_or(shows_rival_gap);
        Some(Self {
            place: race.place,
            racers: race.racers,
            progress: progress_of(&race, info).filter(|_| !drag),
            bar: (more && !drag).then_some(race.progress.clamp(0.0, 1.0)),
            remaining_m: info.and_then(|info| info.remaining_m).filter(|_| more || drag),
            rival: info.and_then(|info| info.rival_gap.clone()).filter(|_| more && !drag && timed),
            figure: figure_of(hud, kind),
        })
    }
}

/// The lap of a lapped race, else the gate of the route; `None` once the player is done.
fn progress_of(race: &RaceHud, info: Option<&RaceInfo>) -> Option<Progress> {
    if let Some((now, of)) = race.lap {
        return Some(Progress::Lap { now, of, last: of > 1 && now == of });
    }
    let info = info?;
    let now = info.gate?;
    (info.gates > 0).then_some(Progress::Gate { now, of: info.gates })
}

/// The figure of a speed-trap or a tollbooth race.
fn figure_of(hud: &HudState, kind: Option<RaceType>) -> Option<Figure> {
    match kind? {
        RaceType::SpeedTrap => hud.context.race_points.map(Figure::Points),
        RaceType::Tollbooth => hud.race.map(|race| Figure::TimeLeft(race.clock)),
        _ => None,
    }
}

/// The strip's height at scale 1, and the widths of its cells.
const HEIGHT: f32 = 24.0;
const PAD: f32 = 8.0;
const SPACE: f32 = 10.0;
const HEADING_W: f32 = 26.0;
const VALUE_W: f32 = 36.0;
const LAP_HEADING_W: f32 = 54.0;
const BAR_W: f32 = 84.0;
const DISTANCE_W: f32 = 56.0;
const NAME_W: f32 = 74.0;
const CHEVRON_W: f32 = 10.0;
const GAP_W: f32 = 92.0;
const FIGURE_W: f32 = 80.0;
/// How far the position value grows at the start of its pulse.
const PULSE_GROWTH: f32 = 0.15;

/// The Custom `RaceStatus` component.
pub struct RaceStrip {
    feed: CustomFeedCell,
    drawn: Drawn,
}

impl RaceStrip {
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, drawn: Drawn::default() }
    }
}

impl Component for RaceStrip {
    /// The strip places itself in the HUD's frame, not in `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let frame = self.feed.memory().frame(&feed);
        self.drawn = draw(&feed, &frame, cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.drawn.build(out);
    }
}

/// The widths of the cells `model` has, at scale 1, in order.
fn cell_widths(model: &StripModel) -> Vec<f32> {
    let mut widths = vec![HEADING_W + VALUE_W];
    if model.progress.is_some() {
        widths.push(LAP_HEADING_W + VALUE_W);
    }
    if model.bar.is_some() {
        widths.push(BAR_W);
    }
    if model.remaining_m.is_some() {
        widths.push(DISTANCE_W);
    }
    if model.figure.is_some() {
        widths.push(FIGURE_W);
    }
    if model.rival.is_some() {
        widths.push(NAME_W + CHEVRON_W + GAP_W);
    }
    widths
}

/// The strip of the frame: in `Racing`, with `race_info` on.
fn draw(feed: &CustomFeed, frame: &Frame, cx: &LayoutCx<'_>) -> Drawn {
    let motion = Motion::for_settings(&feed.settings);
    let fade = frame.opacity(|mode| mode == HudMode::Racing, &motion);
    if fade <= 0.0 || !feed.settings.race_info {
        return Drawn::default();
    }
    let Some(model) = StripModel::of(&feed.hud, Detail::of(&feed.settings)) else { return Drawn::default() };
    let mut parts = Parts::new(cx, HudBox::new(feed.screen, feed.settings.scale), &feed.settings);
    parts.fade(fade);
    let widths = cell_widths(&model);
    let total = widths.iter().sum::<f32>() + SPACE * (widths.len() - 1) as f32 + 2.0 * PAD;
    // It slides down into place as the race starts.
    let arriving = match frame.mode() == HudMode::Racing {
        true => 1.0 - ramp(frame.clock.age(frame.now), motion.slide),
        false => 0.0,
    };
    let rect = parts.place(Anchor::TopCentre, (0.0, -motion.slide_distance * arriving), (total, HEIGHT));
    parts.plate(rect, PlateStyle::SURFACE.with_accent(keys::ACCENT));
    let mut row = Row::new(rect.min_x(), rect.min_y(), rect.h, parts.scale());
    row.skip(PAD);
    place_cell(&mut parts, &mut row, &model, frame, &motion);
    if let Some(progress) = model.progress {
        row.skip(SPACE);
        progress_cell(&mut parts, &mut row, progress);
    }
    if let Some(done) = model.bar {
        row.skip(SPACE);
        let cell = row.cell(BAR_W);
        let s = parts.scale();
        let bar = Rect::new(cell.x, cell.y, cell.w, 6.0 * s);
        parts.segments(bar, SEGMENTS, (done * SEGMENTS as f32).round() as usize, keys::NAV);
    }
    if let Some(metres) = model.remaining_m {
        row.skip(SPACE);
        let label = Label::numeric(distance_long(metres, feed.units), size::STRIP_VALUE).justify(centred());
        parts.text(label, row.cell(DISTANCE_W));
    }
    if let Some(figure) = model.figure {
        row.skip(SPACE);
        let text = match figure {
            Figure::Points(points) => t!("hud-race-points", points = format!("{points:.0}")),
            Figure::TimeLeft(secs) => clock(secs),
        };
        parts.text(Label::numeric(text, size::STRIP_VALUE).justify(centred()), row.cell(FIGURE_W));
    }
    if let Some(gap) = &model.rival {
        row.skip(SPACE);
        rival_cell(&mut parts, &mut row, gap);
    }
    parts.finish()
}

/// `POS 2/6`, the value pulsing once when the place changes.
fn place_cell(parts: &mut Parts<'_>, row: &mut Row, model: &StripModel, frame: &Frame, motion: &Motion) {
    let heading = Label::heading(t!("hud-strip-position").to_uppercase(), size::HEADING).justify(left());
    parts.text(heading, row.cell(HEADING_W));
    // The pulse: a held highlight, which also grows and settles unless motion is reduced.
    let pulse = frame.marks.place.filter(|mark| mark.previous.is_some()).map(|mark| frame.age(mark.since));
    let t = pulse.map(|age| ramp(age, motion.pulse)).unwrap_or(1.0);
    let colour = match t < 1.0 {
        true => keys::ACCENT,
        false => keys::TEXT,
    };
    let grow = match motion.pulse_moves {
        true => 1.0 + PULSE_GROWTH * (1.0 - t),
        false => 1.0,
    };
    let value = format!("{}/{}", model.place, model.racers);
    parts.text(Label::numeric(value, size::STRIP_VALUE * grow).colour(colour).justify(left()), row.cell(VALUE_W));
}

/// `LAP 2/3`, `FINAL LAP 3/3` or `CP 4/12`.
fn progress_cell(parts: &mut Parts<'_>, row: &mut Row, progress: Progress) {
    let (heading, now, of) = match progress {
        Progress::Lap { now, of, last: true } => (t!("hud-strip-final-lap"), now, of),
        Progress::Lap { now, of, last: false } => (t!("hud-strip-lap"), now, of),
        Progress::Gate { now, of } => (t!("hud-strip-checkpoint"), now, of),
    };
    let heading = Label::heading(heading.to_uppercase(), size::HEADING).justify(right());
    let cell = row.cell(LAP_HEADING_W);
    parts.text(heading, Rect::new(cell.x - 3.0 * parts.scale(), cell.y, cell.w, cell.h));
    parts.text(Label::numeric(format!("{now}/{of}"), size::STRIP_VALUE).justify(left()), row.cell(VALUE_W));
}

/// The rival's name, the side's chevron and the gap in words.
fn rival_cell(parts: &mut Parts<'_>, row: &mut Row, gap: &RivalGap) {
    let name = Label::label(gap.name.to_uppercase(), size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY);
    parts.text(name.justify(right()), row.cell(NAME_W));
    let side = Side::of(gap);
    let (key, colour) = match side {
        Side::Ahead => (keys::SUCCESS, keys::SUCCESS),
        Side::Behind => (keys::DANGER, keys::DANGER),
        Side::Level => (keys::TEXT_SECONDARY, keys::TEXT),
    };
    let cell = row.cell(CHEVRON_W);
    if let Some(points) = chevron(cell, side, parts.scale()) {
        parts.stroke(points, 1.5, key);
    }
    parts.text(Label::numeric(gap_words(gap), size::STRIP_VALUE).colour(colour).justify(left()), row.cell(GAP_W));
}

/// An up chevron for ahead, a down one for behind, in the middle of `cell`; none when level.
pub fn chevron(cell: Rect, side: Side, scale: f32) -> Option<Vec<Vec2>> {
    let (w, h) = (3.0 * scale, 2.5 * scale);
    let (x, y) = (cell.x, cell.y);
    match side {
        Side::Ahead => Some(vec![Vec2::new(x - w, y + h), Vec2::new(x, y - h), Vec2::new(x + w, y + h)]),
        Side::Behind => Some(vec![Vec2::new(x - w, y - h), Vec2::new(x, y + h), Vec2::new(x + w, y - h)]),
        Side::Level => None,
    }
}
