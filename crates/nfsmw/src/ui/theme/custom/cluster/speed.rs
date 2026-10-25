//! The speed row: one charcoal plate holding the gear box (drawn by `revs.rs`), the speed in large condensed digits and
//! the unit (`docs/specs/custom-hud/telemetry.md` section 3).
//!
//! The digits are the raw speed floored, exactly as `HudState::display_speed` gives it, so they never lag; only the
//! speed scale under them is smoothed. They are right-aligned on their last cell, so the unit beside them does not
//! move. The unit follows the player's `speed_units` setting.

use blackbox_ui::{Rect, justify};

use super::super::plate::{Plate, PlateStyle};
use super::super::text::{Label, size};
use super::super::tokens::keys;
use super::frame::{Canvas, Frame, SPEED_PAD, WIDTH};
use crate::hud::HudState;
use crate::i18n::t;
use crate::settings::SpeedUnits;

/// The space between the plate's edge and the gear box, and between the parts of the row, at scale 1.
pub const PAD: f32 = 6.0;
/// The gear box's side at text size 1.
pub const GEAR_BOX: f32 = 30.0;
/// The unit's box width at text size 1.
pub const UNIT_WIDTH: f32 = 34.0;
/// How far above the plate's lower padding the unit's box starts, at text size 1: the unit sits low, near the
/// digits' foot.
const UNIT_LIFT: f32 = 6.0;

/// The rectangles of the speed row, in canvas units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedRow {
    /// The plate behind the whole row.
    pub plate: Rect,
    /// The gear box, at the row's left.
    pub gear: Rect,
    /// The speed digits, right-aligned in this box.
    pub digits: Rect,
    /// The unit, right of the digits.
    pub unit: Rect,
}

impl SpeedRow {
    /// The speed row of `frame`.
    pub fn of(frame: &Frame) -> Self {
        let band = frame.rows.speed;
        let t = frame.text_size();
        let side = GEAR_BOX * t;
        let unit_width = UNIT_WIDTH * t;
        let unit_x = WIDTH - PAD - unit_width;
        let digits_x = PAD + side + PAD;
        let digits_width = (unit_x - PAD / 2.0 - digits_x).max(0.0);
        Self {
            plate: frame.band(band),
            gear: frame.rect(PAD, band.bottom + (band.height - side) / 2.0, side, side),
            digits: frame.rect(digits_x, band.bottom, digits_width, band.height),
            unit: frame.rect(unit_x, band.bottom + SPEED_PAD / 2.0 + UNIT_LIFT * t, unit_width, size::HEADING * t),
        }
    }
}

/// The digits the cluster shows for `hud`: whole units, floored, as `display_speed` gives them.
pub fn digits(hud: &HudState) -> String {
    hud.display_speed().to_string()
}

/// The unit's text in the current language.
pub fn unit(units: SpeedUnits) -> String {
    match units {
        SpeedUnits::Metric => t!("hud-cluster-kmh"),
        SpeedUnits::Imperial => t!("hud-cluster-mph"),
    }
}

/// Adds the plate behind the speed row. Drawn first, so everything else in the row sits on it.
pub fn plate(canvas: &mut Canvas<'_>, row: &SpeedRow) {
    let style = PlateStyle::SURFACE.scaled(canvas.frame.scale()).with_opacity(canvas.opacity());
    let plate = Plate::new(row.plate, style, &canvas.cx);
    canvas.push(plate);
}

/// Adds the speed digits and the unit.
pub fn readout(canvas: &mut Canvas<'_>, hud: &HudState, units: SpeedUnits, row: &SpeedRow) {
    let sizing = canvas.frame.sizing;
    let opacity = canvas.opacity();
    let speed = Label::numeric(digits(hud), size::SPEED)
        .justify(justify::H_RIGHT | justify::V_CENTRE)
        .opacity(opacity)
        .laid_out(row.digits, sizing, &canvas.cx);
    canvas.push(speed);
    let label = Label::heading(unit(units), size::HEADING)
        .colour(keys::TEXT_SECONDARY)
        .justify(justify::V_BOTTOM)
        .opacity(opacity)
        .laid_out(row.unit, sizing, &canvas.cx);
    canvas.push(label);
}
