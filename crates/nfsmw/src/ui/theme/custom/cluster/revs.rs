//! The revs and the gear (`docs/specs/custom-hud/telemetry.md` section 5): a compact rev strip above the speed row,
//! the gear box at the row's left, and the shift cue.
//!
//! - **Rev strip:** [`SEGMENTS`] segments from 0 to `max_rpm`, in fixed bands. A segment that reaches past the amber
//!   start is amber (`caution`), one that reaches past the red line is red (`danger`), the rest are `text_secondary`
//!   when lit; an unlit segment is `rule`. The amber start is `shift_up_rpm`, or [`AMBER_OF_RED_LINE`] of the red line
//!   when the gearbox reports no shift point. The amber and red segments are also taller, so the bands read by shape.
//! - **Gear box:** `R`, `N` or the gear's number in a square edged in the accent; the digit is dimmed to
//!   [`SHIFTING_ALPHA`] while a shift is in progress.
//! - **Shift cue:** when the gearbox wants the next gear (`shift_light`), or the player drives a manual box and the revs
//!   reached `shift_up_rpm`. The box fills with `caution`, the digit turns dark and an up-chevron sits above the box.
//!   It is steady while the condition holds; one pulse marks its start (the edge turns `text`, and with motion the
//!   chevron rises and settles).

use blackbox_ui::{LineJoin, Outline, Polyline, justify};
use glam::Vec2;

use super::super::plate::RULE_WIDTH;
use super::super::text::{Label, size};
use super::super::tokens::keys;
use super::frame::{Canvas, TALL, WIDTH, lit, mix};
use super::speed::SpeedRow;
use crate::hud::HudState;
use crate::i18n::t;
use crate::settings::Transmission;

/// The segments of the rev strip.
pub const SEGMENTS: usize = 20;
/// The space between two rev segments at scale 1.
pub const SEGMENT_GAP: f32 = 2.0;
/// Where the amber band starts without a shift point, as a share of the red line.
pub const AMBER_OF_RED_LINE: f32 = 0.85;
/// The gear digit's alpha while a shift is in progress.
pub const SHIFTING_ALPHA: f32 = 0.45;
/// The chevron's half width and height at scale 1, its gap above the box, and how far the start pulse lifts it.
const CHEVRON_HALF_WIDTH: f32 = 5.0;
const CHEVRON_HEIGHT: f32 = 4.0;
const CHEVRON_GAP: f32 = 2.0;
const CHEVRON_LIFT: f32 = 2.0;
/// The chevron's stroke at scale 1.
const CHEVRON_STROKE: f32 = 1.5;

/// The band a rev segment is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Normal,
    Amber,
    Red,
}

impl Band {
    /// The colour key of a lit segment in the band.
    pub fn key(self) -> &'static str {
        match self {
            Band::Normal => keys::TEXT_SECONDARY,
            Band::Amber => keys::CAUTION,
            Band::Red => keys::DANGER,
        }
    }

    /// The segment's height over an ordinary one's.
    pub fn height(self) -> f32 {
        match self {
            Band::Normal => 1.0,
            Band::Amber | Band::Red => TALL,
        }
    }
}

/// Where the amber band starts for `hud`: the shift point, or [`AMBER_OF_RED_LINE`] of the red line without one.
pub fn amber_start(hud: &HudState) -> f32 {
    hud.telemetry.shift_up_rpm.unwrap_or(AMBER_OF_RED_LINE * hud.red_line)
}

/// The band of a segment whose top end is at `top` rpm: red when it reaches past the red line, amber when it reaches
/// past the amber start, normal otherwise.
pub fn band(top: f32, amber: f32, red_line: f32) -> Band {
    if top > red_line {
        return Band::Red;
    }
    if top > amber {
        return Band::Amber;
    }
    Band::Normal
}

/// The band of rev segment `index` for `hud`.
pub fn segment_band(index: usize, hud: &HudState) -> Band {
    let top = (index + 1) as f32 / SEGMENTS as f32 * hud.max_rpm;
    band(top, amber_start(hud), hud.red_line)
}

/// The share of the strip the revs fill, 0 to 1. An engine with no maximum fills none of it.
pub fn rev_fraction(hud: &HudState) -> f32 {
    if hud.max_rpm.is_nan() || hud.max_rpm <= 0.0 || hud.rpm.is_nan() {
        return 0.0;
    }
    (hud.rpm / hud.max_rpm).clamp(0.0, 1.0)
}

/// Whether the shift cue shows: the gearbox wants the next gear, or the player drives a manual box and the revs reached
/// the shift point.
pub fn shift_cue(hud: &HudState, transmission: Transmission) -> bool {
    if hud.shift_light {
        return true;
    }
    if transmission != Transmission::Manual {
        return false;
    }
    hud.telemetry.shift_up_rpm.is_some_and(|up| hud.rpm >= up)
}

/// The gear's text: `R`, `N` or the number.
pub fn gear_text(gear: i32) -> String {
    match gear {
        g if g < 0 => t!("hud-cluster-gear-reverse"),
        0 => t!("hud-cluster-gear-neutral"),
        g => g.to_string(),
    }
}

/// Adds the rev strip above the speed row.
pub fn strip(canvas: &mut Canvas<'_>, hud: &HudState) {
    let row = canvas.frame.rows.revs;
    let fraction = rev_fraction(hud);
    let off = canvas.paint.colour(keys::RULE);
    let width = (WIDTH - SEGMENT_GAP * (SEGMENTS as f32 - 1.0)) / SEGMENTS as f32;
    for index in 0..SEGMENTS {
        let band = segment_band(index, hud);
        let on = canvas.paint.colour(band.key());
        let x = index as f32 * (width + SEGMENT_GAP);
        let rect = canvas.frame.rect(x, row.bottom, width, row.height * band.height());
        canvas.panel(mix(off, on, lit(index, SEGMENTS, fraction)), rect);
    }
}

/// Adds the gear box, with the shift cue when `cue` is on and its start pulse at `pulse` (0 to 1).
pub fn gear(canvas: &mut Canvas<'_>, hud: &HudState, cue: bool, pulse: f32, row: &SpeedRow) {
    let scale = canvas.frame.scale();
    if cue {
        let fill = canvas.paint.colour(keys::CAUTION);
        canvas.panel(fill, row.gear);
    }
    let edge = match pulse > 0.0 {
        true => keys::TEXT,
        false => keys::ACCENT,
    };
    let outline = Outline::new(edge, row.gear, 0.0, RULE_WIDTH * scale, &canvas.paint.cx());
    canvas.push(outline);
    // On the cue's fill the digit is dark, so it reads on the amber.
    let digit_key = match cue {
        true => keys::CRITICAL_TEXT,
        false => keys::TEXT,
    };
    let dim = match hud.shifting {
        true => SHIFTING_ALPHA,
        false => 1.0,
    };
    let digit = Label::numeric(gear_text(hud.gear), size::GEAR)
        .colour(digit_key)
        .justify(justify::H_CENTRE | justify::V_CENTRE)
        .opacity(canvas.opacity() * dim)
        .laid_out(row.gear, canvas.frame.sizing, &canvas.cx);
    canvas.push(digit);
    if !cue {
        return;
    }
    let lift = match canvas.motion.pulse_moves {
        true => CHEVRON_LIFT * pulse,
        false => 0.0,
    };
    let base = row.gear.min_y() - (CHEVRON_GAP + lift) * scale;
    let (x, half, height) = (row.gear.x, CHEVRON_HALF_WIDTH * scale, CHEVRON_HEIGHT * scale);
    let points = vec![Vec2::new(x - half, base), Vec2::new(x, base - height), Vec2::new(x + half, base)];
    let colour = canvas.paint.colour(keys::CAUTION);
    let chevron = Polyline::new(points, CHEVRON_STROKE * scale, colour).with_join(LineJoin::Mitre { limit: 2.0 });
    canvas.push(chevron);
}
