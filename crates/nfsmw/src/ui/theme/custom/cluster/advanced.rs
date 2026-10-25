//! The advanced row above the rev strip (`docs/specs/custom-hud/telemetry.md` section 8), shown with
//! `[custom_hud] advanced_telemetry` (which the Advanced preset turns on): the rpm, the boost in psi, the engine load,
//! the lateral acceleration and the tyres.
//!
//! Each field is drawn only when the game has its value, and the fields that are drawn close up to the cluster's right
//! edge, so a missing metric leaves no gap and no placeholder:
//!
//! | Field | Shown when |
//! |---|---|
//! | rpm, to the nearest [`RPM_STEP`] | always |
//! | boost psi | `has_turbo` |
//! | engine load (`telemetry.throttle`) | always: the powertrain reports it every frame |
//! | lateral g (`telemetry.lateral_g`, its size) | always: the simulation derives it every frame |
//! | tyres | `condition` is known |
//!
//! The tyres are four marks in the car's layout (front left, front right; rear left, rear right). Shape carries the
//! state as well as colour: a solid mark is sound, a notched one punctured, a hollow one blown. Engine and brake
//! temperatures are not shown: the simulation does not compute them.

use blackbox_ui::{Outline, justify};

use super::super::plate::RULE_WIDTH;
use super::super::text::Label;
use super::super::tokens::keys;
use super::frame::{ADVANCED_TEXT, Canvas, WIDTH};
use crate::hud::{HudState, TireCondition};
use crate::i18n::t;

/// The rpm is shown to the nearest this many.
pub const RPM_STEP: f32 = 10.0;
/// The gap between two fields at scale 1.
const FIELD_GAP: f32 = 8.0;
/// The text fields' widths at text size 1.
const RPM_WIDTH: f32 = 54.0;
const PSI_WIDTH: f32 = 48.0;
const LOAD_WIDTH: f32 = 34.0;
const LATERAL_WIDTH: f32 = 36.0;
/// A tyre mark's side and the gap between marks, at scale 1.
const MARK: f32 = 4.0;
const MARK_GAP: f32 = 2.0;

/// One field of the advanced row.
#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    /// A value as text, and its box width at text size 1.
    Text(String, f32),
    /// The tyres, in wheel order: front left, front right, rear left, rear right.
    Tyres([TireCondition; 4]),
}

impl Field {
    /// The field's width at scale 1 and text size `t`.
    fn width(&self, t: f32) -> f32 {
        match self {
            Field::Text(_, width) => width * t,
            Field::Tyres(_) => 2.0 * MARK + MARK_GAP,
        }
    }
}

/// The fields of the advanced row for `hud`, left to right; a value the game does not have is left out.
pub fn fields(hud: &HudState) -> Vec<Field> {
    let mut fields = vec![Field::Text(rpm_text(hud.rpm), RPM_WIDTH)];
    if hud.has_turbo {
        fields.push(Field::Text(t!("hud-cluster-psi", psi = format!("{:.1}", hud.boost_psi)), PSI_WIDTH));
    }
    fields.push(Field::Text(t!("hud-cluster-load", load = load_percent(hud.telemetry.throttle)), LOAD_WIDTH));
    let lateral = format!("{:.1}", hud.telemetry.lateral_g.abs());
    fields.push(Field::Text(t!("hud-cluster-lateral-g", g = lateral), LATERAL_WIDTH));
    if let Some(condition) = &hud.condition {
        fields.push(Field::Tyres(condition.tires));
    }
    fields
}

/// The rpm's text: to the nearest [`RPM_STEP`].
pub fn rpm_text(rpm: f32) -> String {
    let rounded = (rpm.max(0.0) / RPM_STEP).round() * RPM_STEP;
    t!("hud-cluster-rpm", rpm = rounded as u32)
}

/// The engine load as a whole percentage, 0 to 100.
pub fn load_percent(throttle: f32) -> u32 {
    if throttle.is_nan() {
        return 0;
    }
    (throttle.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// Adds the advanced row's fields, closed up to the cluster's right edge.
pub fn draw(canvas: &mut Canvas<'_>, hud: &HudState) {
    let fields = fields(hud);
    let row = canvas.frame.rows.advanced;
    let t = canvas.frame.text_size();
    let mut right = WIDTH;
    for field in fields.iter().rev() {
        let width = field.width(t);
        let x = right - width;
        right = x - FIELD_GAP;
        match field {
            Field::Text(text, _) => {
                let text = Label::numeric(text.clone(), ADVANCED_TEXT)
                    .colour(keys::TEXT_SECONDARY)
                    .justify(justify::H_RIGHT | justify::V_CENTRE)
                    .opacity(canvas.opacity())
                    .laid_out(canvas.frame.rect(x, row.bottom, width, row.height), canvas.frame.sizing, &canvas.cx);
                canvas.push(text);
            }
            Field::Tyres(tyres) => {
                tyre_marks(canvas, tyres, x, row.bottom + (row.height - (2.0 * MARK + MARK_GAP)) / 2.0)
            }
        }
    }
}

/// Adds the four tyre marks in a two by two block whose bottom-left corner is at `x`, `bottom` (scale 1).
fn tyre_marks(canvas: &mut Canvas<'_>, tyres: &[TireCondition; 4], x: f32, bottom: f32) {
    let step = MARK + MARK_GAP;
    // Front left, front right on the upper row; rear left, rear right on the lower one.
    let places = [(0.0, step), (step, step), (0.0, 0.0), (step, 0.0)];
    for (tyre, (dx, dy)) in tyres.iter().zip(places) {
        tyre_mark(canvas, *tyre, x + dx, bottom + dy);
    }
}

/// Adds one tyre mark: a solid square when sound, a notched one when punctured, a hollow one when blown.
fn tyre_mark(canvas: &mut Canvas<'_>, tyre: TireCondition, x: f32, bottom: f32) {
    match tyre {
        TireCondition::Sound => {
            let colour = canvas.paint.colour(keys::TEXT_SECONDARY);
            let rect = canvas.frame.rect(x, bottom, MARK, MARK);
            canvas.panel(colour, rect);
        }
        TireCondition::Punctured => {
            // The right half is cut down: a notch out of the top-right corner.
            let colour = canvas.paint.colour(keys::CAUTION);
            let left = canvas.frame.rect(x, bottom, MARK / 2.0, MARK);
            let right = canvas.frame.rect(x + MARK / 2.0, bottom, MARK / 2.0, MARK / 2.0);
            canvas.panel(colour, left);
            canvas.panel(colour, right);
        }
        TireCondition::Blown => {
            let rect = canvas.frame.rect(x, bottom, MARK, MARK);
            let thickness = RULE_WIDTH * canvas.frame.scale();
            let ring = Outline::new(keys::DANGER, rect, 0.0, thickness, &canvas.paint.cx());
            canvas.push(ring);
        }
    }
}
