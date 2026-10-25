//! The speed scale: a row of [`SEGMENTS`] segments under the digits (`docs/specs/custom-hud/telemetry.md` section 4).
//!
//! The lit share is the presented speed over the scale's top, [`TOP_KMH`] or [`TOP_MPH`]; past the top every segment
//! is lit. The presented speed is the raw speed through a critically damped spring that settles in about
//! `Motion::speed_spring` (60 ms), stepped on the frame time (`memory.rs`): it takes out the jumps of the physics step
//! without a visible lag. A lit segment is `text`, an empty one `rule`, and the segment under the fill's end is mixed
//! between the two by how much of it the fill covers.

use super::super::tokens::keys;
use super::frame::{Canvas, lit, mix, segments};
use crate::settings::SpeedUnits;

/// The segments of the scale.
pub const SEGMENTS: usize = 24;
/// The space between two segments at scale 1.
pub const SEGMENT_GAP: f32 = 1.5;
/// The speed at which the scale is full, in kilometres per hour and in miles per hour.
pub const TOP_KMH: f32 = 360.0;
pub const TOP_MPH: f32 = 225.0;
/// Kilometres per hour and miles per hour in one metre per second (`HudState::display_speed` uses the same factors).
pub const KMH_PER_MPS: f32 = 3.6;
pub const MPH_PER_MPS: f32 = 2.236_936;

/// One metre per second in `units`.
pub fn per_mps(units: SpeedUnits) -> f32 {
    match units {
        SpeedUnits::Metric => KMH_PER_MPS,
        SpeedUnits::Imperial => MPH_PER_MPS,
    }
}

/// The speed at which the scale is full, in `units`.
pub fn top(units: SpeedUnits) -> f32 {
    match units {
        SpeedUnits::Metric => TOP_KMH,
        SpeedUnits::Imperial => TOP_MPH,
    }
}

/// The share of the scale lit by `speed` metres per second read in `units`, 0 to 1.
pub fn fraction(speed: f32, units: SpeedUnits) -> f32 {
    if speed.is_nan() {
        return 0.0;
    }
    (speed.max(0.0) * per_mps(units) / top(units)).clamp(0.0, 1.0)
}

/// Adds the scale's segments for the presented speed `speed` (metres per second) read in `units`.
pub fn draw(canvas: &mut Canvas<'_>, speed: f32, units: SpeedUnits) {
    let rect = canvas.frame.band(canvas.frame.rows.scale);
    let fraction = fraction(speed, units);
    let (on, off) = (canvas.paint.colour(keys::TEXT), canvas.paint.colour(keys::RULE));
    let gap = SEGMENT_GAP * canvas.frame.scale();
    for (index, segment) in segments(rect, SEGMENTS, gap).into_iter().enumerate() {
        canvas.panel(mix(off, on, lit(index, SEGMENTS, fraction)), segment);
    }
}
