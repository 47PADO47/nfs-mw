//! Paint cooling: the heat factor of each visual part a career car is fitted with, and the heat meter that shows it
//! (`docs/specs/career-cooling.md`).
//!
//! The data is the `fecooling` collection of the attribute database (eleven floats below 1); this crate takes it as a
//! plain [`CoolingTable`] so it stays free of the install readers. The cooling runs when a part is fitted in the
//! customization garage of a career, not during a chase.

use crate::stable::CareerRecord;

/// The factor shop visits use ("extra adjust" 1.0).
pub const SHOP_EXTRA: f32 = 1.0;
/// The extra factor of the backroom shop (0.75), so a part fitted there cools more.
pub const BACKROOM_EXTRA: f32 = 0.75;

/// The kinds of visual part that cool the car (the `fecooling` fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingPart {
    BodyKit,
    Decal,
    Hood,
    Numbers,
    Paint,
    Rim,
    RimPaint,
    RoofScoop,
    Spoiler,
    Vinyl,
    WindowTint,
}

impl CoolingPart {
    /// Every kind, in the order of the table.
    pub const ALL: [CoolingPart; 11] = [
        CoolingPart::BodyKit,
        CoolingPart::Decal,
        CoolingPart::Hood,
        CoolingPart::Numbers,
        CoolingPart::Paint,
        CoolingPart::Rim,
        CoolingPart::RimPaint,
        CoolingPart::RoofScoop,
        CoolingPart::Spoiler,
        CoolingPart::Vinyl,
        CoolingPart::WindowTint,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// The eleven factors, indexed by [`CoolingPart`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoolingTable {
    factors: [f32; 11],
}

impl CoolingTable {
    /// A table from the factors in the order of [`CoolingPart::ALL`].
    pub const fn new(factors: [f32; 11]) -> CoolingTable {
        CoolingTable { factors }
    }

    /// The factor of one part kind.
    pub const fn factor(&self, part: CoolingPart) -> f32 {
        self.factors[part.index()]
    }
}

impl CareerRecord {
    /// A part of kind `part` was fitted: `heat *= factor * extra` (the shop passes [`SHOP_EXTRA`], the backroom
    /// [`BACKROOM_EXTRA`]). A heat that is not a number or is negative is left alone.
    pub fn cool_for_part(&mut self, table: &CoolingTable, part: CoolingPart, extra: f32) {
        if self.heat.is_nan() || self.heat < 0.0 {
            return;
        }
        self.heat *= table.factor(part) * extra;
    }
}

/// The heat a car would have after fitting every part of `parts` (the garage's preview): each one multiplies in turn.
pub fn preview_heat(heat: f32, table: &CoolingTable, parts: &[CoolingPart], extra: f32) -> f32 {
    let mut record = CareerRecord::new(0);
    record.heat = heat;
    for part in parts {
        record.cool_for_part(table, *part, extra);
    }
    record.heat
}

/// The lowest heat the meter shows (and the lowest multiplier).
pub const METER_MIN: f32 = 1.0;
/// The highest heat the meter shows.
pub const METER_MAX: f32 = 5.0;
/// Segments of the meter's ring.
pub const METER_SEGMENTS: usize = 10;

/// How one segment of the meter looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    /// The car's heat does not reach this segment.
    Off,
    /// The car's heat reaches it but the preview's does not: the part about to be fitted will remove it.
    Cool,
    /// Lit, and stays lit.
    On,
}

/// The heat meter of the car select and customization screens (`HEAT_X<n>`, ten `HEAT_BASE_LED` segments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatMeter {
    /// The multiplier icon: the preview's whole heat, 1 to 5.
    pub multiplier: u8,
    /// The segments, index 0 the lowest (the first LED), each a tenth of a heat level.
    pub segments: [Segment; METER_SEGMENTS],
}

impl HeatMeter {
    /// The meter for a car with heat `current` and preview heat `preview`. Both are clamped to 1..=5, so a car with no
    /// heat reads as heat 1. The multiplier and the segments' base follow the preview.
    pub fn new(current: f32, preview: f32) -> HeatMeter {
        let clamp = |heat: f32| match heat.is_nan() {
            true => METER_MIN,
            false => heat.clamp(METER_MIN, METER_MAX),
        };
        let (current, preview) = (clamp(current), clamp(preview));
        let mut bottom = METER_MIN;
        let mut multiplier = 1u8;
        while preview - bottom >= 1.0 {
            multiplier += 1;
            bottom += 1.0;
        }
        let multiplier = multiplier.min(5);
        let mut segments = [Segment::On; METER_SEGMENTS];
        for (i, segment) in segments.iter_mut().enumerate() {
            let at = bottom + 0.1 * i as f32;
            *segment = match (at + 0.0005 >= current, at + 0.0005 >= preview) {
                (true, _) => Segment::Off,
                (false, true) => Segment::Cool,
                (false, false) => Segment::On,
            };
        }
        HeatMeter { multiplier, segments }
    }

    /// The meter that shows just the car's heat (no preview).
    pub fn of(heat: f32) -> HeatMeter {
        HeatMeter::new(heat, heat)
    }
}

#[cfg(test)]
#[path = "cooling_tests.rs"]
mod tests;
