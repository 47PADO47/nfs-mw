//! The race's intro card and countdown (`docs/specs/custom-hud/race.md` section 2), drawn in the `Banner` slot.
//!
//! - **Intro card**, top-left under the radio card's zone, so the start lights and the road stay clear: the race type,
//!   the track's name where the language has one, the laps of a lapped race, and the opponents. It slides in with the
//!   intro and leaves when the countdown starts. It is a view of the race's own intro timing, never a timer.
//! - **Countdown**, top-centre in the upper third: large digits `3`, `2`, `1`, then the game's `GO`, one pulse per step
//!   (a held highlight under motion reduction), from `RaceHud::count`.

use crate::hud::{Count, RaceIntro};
use crate::i18n::t;

use super::super::layout::Anchor;
use super::super::parts::{Parts, centred, edges, left};
use super::super::plate::PlateStyle;
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys, ramp};
use super::super::words::race_type;

/// The card's width, a line's height and the padding, at scale 1.
const WIDTH: f32 = 220.0;
const LINE: f32 = 14.0;
const PAD: f32 = 6.0;
/// The countdown's box at scale 1, and how far down from the safe area's top its centre sits (the upper third).
const COUNT_BOX: (f32, f32) = (120.0, 70.0);
pub const COUNT_DOWN: f32 = 96.0;
/// The countdown digits' size at scale 1, and how far a step grows at the start of its pulse.
const COUNT_SIZE: f32 = 56.0;
const COUNT_GROWTH: f32 = 0.25;

/// The lines of the intro card, from the top: the type, the name, the laps and opponents, the opponents' names.
pub fn intro_lines(intro: &RaceIntro) -> Vec<String> {
    let mut lines = vec![race_type(intro.kind).to_uppercase()];
    lines.extend(intro.name.clone());
    let mut facts = Vec::new();
    facts.extend(intro.laps.map(|laps| t!("hud-intro-laps", laps = laps)));
    if !intro.opponents.is_empty() {
        facts.push(t!("hud-intro-opponents", n = intro.opponents.len()));
    }
    if !facts.is_empty() {
        lines.push(facts.join(" \u{b7} "));
    }
    if !intro.opponents.is_empty() {
        lines.push(intro.opponents.join(", "));
    }
    lines
}

/// The intro card at `top` units down the top-left column, `age` seconds into the intro.
pub fn intro_card(parts: &mut Parts<'_>, intro: &RaceIntro, top: f32, age: f32, motion: &Motion) {
    let lines = intro_lines(intro);
    let height = LINE * lines.len() as f32 + 2.0 * PAD;
    let slide = -motion.slide_distance * (1.0 - ramp(age, motion.slide));
    let rect = parts.place(Anchor::TopLeft, (slide, top), (WIDTH, height));
    parts.plate(rect, PlateStyle::SURFACE.with_accent(keys::ACCENT));
    let s = parts.scale();
    for (i, line) in lines.into_iter().enumerate() {
        let y = rect.min_y() + (PAD + i as f32 * LINE) * s;
        let cell = edges(rect.min_x() + PAD * s, y, rect.max_x() - PAD * s, y + LINE * s);
        let label = match i {
            0 => Label::heading(line, size::HEADING).colour(keys::ACCENT),
            1 => Label::label(line, size::NOTIFICATION_TITLE),
            _ => Label::label(line, size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY),
        };
        parts.text(label.justify(left()), cell);
    }
}

/// The countdown's step `count`, `age` seconds after it came up.
pub fn countdown(parts: &mut Parts<'_>, count: Count, age: f32, motion: &Motion) {
    let t = ramp(age, motion.pulse);
    let grow = match motion.pulse_moves {
        true => 1.0 + COUNT_GROWTH * (1.0 - t),
        false => 1.0,
    };
    let colour = match (count, t < 1.0) {
        (Count::Go, _) => keys::SUCCESS,
        (_, true) => keys::ACCENT,
        (_, false) => keys::TEXT,
    };
    let rect = parts.place_clear(Anchor::TopCentre, (0.0, COUNT_DOWN - COUNT_BOX.1 / 2.0), COUNT_BOX);
    parts.text(Label::numeric(count.text(), COUNT_SIZE * grow).colour(colour).justify(centred()), rect);
}
