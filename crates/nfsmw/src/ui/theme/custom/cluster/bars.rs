//! The bar row under the speed scale (`docs/specs/custom-hud/telemetry.md` sections 6 and 7): nitrous, the
//! speedbreaker and boost, each a short label and a row of segments. A bar the car has no part for is not drawn, and
//! the bars that are drawn keep their order and close up to the cluster's right edge, so the row stays aligned with the
//! rest of the cluster.
//!
//! - Nitrous is built in `nitrous.rs` (its activation pulse).
//! - **Speedbreaker:** [`SPEEDBREAKER_SEGMENTS`] segments, `nav` while active and `text` otherwise. In free roam it
//!   hides once it has rested full and unused for `memory::SPEEDBREAKER_HIDE_S`, and returns as soon as it is used or
//!   drains; in races and pursuits it stays.
//! - **Boost:** only with `has_turbo`; [`BOOST_SEGMENTS`] segments from 0 to [`BOOST_FULL_PSI`], a display scale and not
//!   a limit. Vacuum (negative psi) shows an empty bar; the number is in the advanced row.

use blackbox_ui::justify;

use super::super::text::{Label, size};
use super::super::tokens::{fade, keys};
use super::frame::{Canvas, WIDTH, lit, mix, segments};
use super::memory::Presented;
use crate::hud::HudState;
use crate::i18n::t;

/// The speedbreaker's segments.
pub const SPEEDBREAKER_SEGMENTS: usize = 10;
/// The boost bar's segments, and the boost at which it is full.
pub const BOOST_SEGMENTS: usize = 5;
pub const BOOST_FULL_PSI: f32 = 30.0;
/// The alpha of a lit bar segment outside a pulse: a pulse brightens it to full.
pub const LIT_ALPHA: f32 = 0.8;
/// The bars' slots across the cluster: three, this far apart at scale 1.
const SLOTS: usize = 3;
const SLOT_GAP: f32 = 8.0;
/// A label's box at text size 1, and the gap between it and its segments.
const LABEL_WIDTH: f32 = 26.0;
const LABEL_GAP: f32 = 3.0;
/// A segment's height at scale 1, its gap, and how much taller a full pulse makes it.
const SEGMENT_HEIGHT: f32 = 4.0;
const SEGMENT_GAP: f32 = 1.5;
const PULSE_GROWTH: f32 = 0.5;

/// One bar to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    /// The label, in the current language.
    pub label: String,
    /// The label's colour key.
    pub label_key: &'static str,
    /// How many segments the bar has.
    pub segments: usize,
    /// How full it is, 0 to 1.
    pub fill: f32,
    /// A lit segment's colour, already faded.
    pub lit: [u8; 4],
    /// How much taller than usual the segments are, 0 for their usual height (a moving pulse grows them).
    pub grow: f32,
}

/// The speedbreaker's bar, or `None` while it rests hidden in free roam.
pub fn speedbreaker(canvas: &Canvas<'_>, hud: &HudState, presented: &Presented) -> Option<Bar> {
    if presented.speedbreaker_hidden {
        return None;
    }
    let key = match hud.speedbreaker_active {
        true => keys::NAV,
        false => keys::TEXT,
    };
    Some(Bar {
        label: t!("hud-cluster-speedbreaker"),
        label_key: keys::TEXT_SECONDARY,
        segments: SPEEDBREAKER_SEGMENTS,
        fill: hud.speedbreaker.clamp(0.0, 1.0),
        lit: fade(canvas.paint.colour(key), LIT_ALPHA),
        grow: 0.0,
    })
}

/// The boost bar, or `None` for a car without forced induction.
pub fn boost(canvas: &Canvas<'_>, hud: &HudState) -> Option<Bar> {
    if !hud.has_turbo {
        return None;
    }
    Some(Bar {
        label: t!("hud-cluster-boost"),
        label_key: keys::TEXT_SECONDARY,
        segments: BOOST_SEGMENTS,
        fill: boost_fill(hud.boost_psi),
        lit: fade(canvas.paint.colour(keys::TEXT), LIT_ALPHA),
        grow: 0.0,
    })
}

/// The boost bar's fill for `psi`: 0 at or below 0 psi, full from [`BOOST_FULL_PSI`].
pub fn boost_fill(psi: f32) -> f32 {
    if psi.is_nan() {
        return 0.0;
    }
    (psi / BOOST_FULL_PSI).clamp(0.0, 1.0)
}

/// Adds `bars`, in order, closed up to the cluster's right edge.
pub fn draw(canvas: &mut Canvas<'_>, bars: &[Bar]) {
    let row = canvas.frame.rows.bars;
    let t = canvas.frame.text_size();
    let scale = canvas.frame.scale();
    let slot_width = (WIDTH - SLOT_GAP * (SLOTS as f32 - 1.0)) / SLOTS as f32;
    let first = SLOTS.saturating_sub(bars.len());
    for (slot, bar) in (first..SLOTS).zip(bars) {
        let x = slot as f32 * (slot_width + SLOT_GAP);
        let label_width = (LABEL_WIDTH * t).min(slot_width / 2.0);
        let label = Label::heading(bar.label.clone(), size::HEADING)
            .colour(bar.label_key)
            .justify(justify::V_CENTRE)
            .opacity(canvas.opacity())
            .laid_out(canvas.frame.rect(x, row.bottom, label_width, row.height), canvas.frame.sizing, &canvas.cx);
        canvas.push(label);
        let bar_x = x + label_width + LABEL_GAP;
        let bar_width = (slot_width - label_width - LABEL_GAP).max(0.0);
        let height = SEGMENT_HEIGHT * (1.0 + PULSE_GROWTH * bar.grow.clamp(0.0, 1.0));
        let bottom = row.bottom + (row.height - height) / 2.0;
        let track = canvas.frame.rect(bar_x, bottom, bar_width, height);
        let off = canvas.paint.colour(keys::RULE);
        for (index, segment) in segments(track, bar.segments, SEGMENT_GAP * scale).into_iter().enumerate() {
            canvas.panel(mix(off, bar.lit, lit(index, bar.segments, bar.fill)), segment);
        }
    }
}
