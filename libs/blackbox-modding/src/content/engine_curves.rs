//! The curve part of a layer of `[engine_sound]`: what a layer's volume and pitch follow.
//!
//! A layer with a `gain` list plays by curves instead of by the engine speed it was recorded at: its volume is the
//! product of its curves (each is read at one live input of the car) times its `volume`, and its pitch is a fixed
//! speed or follows an input.
//!
//! ```toml
//! { file = "SOUND/MODENGINE/MYCAR/G0.GIN", kind = "ginsu", span = [1790.0, 7500.0], volume = 0.7,
//!   gain = [ { input = "rpm", points = [[0.0, 0.0], [0.02, 1.0], [1.0, 1.0]] },
//!            { input = "load", points = [[0.0, 0.0], [1.0, 1.0]] } ] }
//! { file = "SOUND/MODENGINE/MYCAR/W0.WAV", gain = [ { input = "speed_mph", points = [[10.0, 0.0], [40.0, 1.0]] } ],
//!   pitch = { input = "speed_mph", reference = 40.0 } }
//! { file = "SOUND/MODENGINE/MYCAR/M0.WAV", gain = [ { input = "speed_mph", points = [[0.0, 1.0], [150.0, 1.0]] } ],
//!   rate = { input = "speed_mph", points = [[0.0, 1.0], [60.0, 2.0], [150.0, 2.2]] } }
//! ```
//!
//! `pitch` plays a layer at an input over a reference; `rate` is a curve of an input that gives the playback rate itself
//! (an electric motor's whine rises quickly at first, then levels off), and takes the place of `pitch`.

use serde::Deserialize;

use super::ContentError;

/// Most curves a layer may multiply.
pub const MAX_CURVES: usize = 4;
/// Fewest and most points of a curve.
pub const POINTS: std::ops::RangeInclusive<usize> = 2..=32;
/// The largest value of a curve point and of a layer's volume.
pub const MAX_LEVEL: f32 = 8.0;
/// The range of a pitch reference.
pub const REFERENCE_RANGE: std::ops::RangeInclusive<f32> = 0.01..=100_000.0;

/// What a curve is read at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    /// How far the engine is from idle (0) to redline (1).
    Rpm,
    /// The engine load, the throttle, 0 to 1.
    Load,
    /// The car's speed in miles per hour.
    SpeedMph,
}

/// What a layer plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    /// A looping WAV.
    #[default]
    Loop,
    /// A granular engine recording (`.GIN`): it is played at the frequency that `span` maps the engine speed to.
    Ginsu,
}

/// A piecewise linear curve of one input; flat before its first and after its last point.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub input: Input,
    pub points: Vec<(f32, f32)>,
}

impl Curve {
    /// The curve at `x`.
    pub fn eval(&self, x: f32) -> f32 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else { return 1.0 };
        if x <= first.0 {
            return first.1;
        }
        if x >= last.0 {
            return last.1;
        }
        let Some(pair) = self.points.windows(2).find(|pair| x <= pair[1].0) else { return last.1 };
        let t = (x - pair[0].0) / (pair[1].0 - pair[0].0);
        pair[0].1 + (pair[1].1 - pair[0].1) * t
    }
}

/// A pitch that follows an input: the playback rate is the input over `reference`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
    pub input: Input,
    pub reference: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawCurve {
    input: Input,
    points: Vec<[f32; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPitch {
    input: Input,
    reference: f32,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// Check a layer's curves: 1 to 4 curves of 2 to 32 points with rising `x` and `y` from 0 to 8.
pub(super) fn check_curves(raw: Vec<RawCurve>) -> Result<Vec<Curve>, ContentError> {
    if raw.len() > MAX_CURVES {
        return Err(field("engine_sound.layers.gain", "a layer has at most 4 curves"));
    }
    let mut curves = Vec::with_capacity(raw.len());
    for curve in raw {
        if !POINTS.contains(&curve.points.len()) {
            return Err(field("engine_sound.layers.gain", "a curve has 2 to 32 points"));
        }
        let finite = curve.points.iter().all(|point| point[0].is_finite() && point[1].is_finite());
        if !finite || curve.points.iter().any(|point| !(0.0..=MAX_LEVEL).contains(&point[1])) {
            return Err(field("engine_sound.layers.gain", "a curve point's value is a number from 0 to 8"));
        }
        if curve.points.windows(2).any(|pair| pair[0][0] >= pair[1][0]) {
            return Err(field("engine_sound.layers.gain", "the points of a curve rise in x"));
        }
        curves.push(Curve {
            input: curve.input,
            points: curve.points.iter().map(|point| (point[0], point[1])).collect(),
        });
    }
    Ok(curves)
}

/// Check a layer's pitch rule.
pub(super) fn check_pitch(raw: Option<RawPitch>) -> Result<Option<Pitch>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    if !REFERENCE_RANGE.contains(&raw.reference) {
        return Err(field("engine_sound.layers.pitch", "a pitch reference is a number from 0.01 to 100000"));
    }
    Ok(Some(Pitch { input: raw.input, reference: raw.reference }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curve_is_linear_between_its_points_and_flat_outside() {
        let curve = Curve { input: Input::Load, points: vec![(0.2, 0.0), (0.6, 1.0), (1.0, 0.5)] };
        assert_eq!((curve.eval(0.0), curve.eval(0.2), curve.eval(2.0)), (0.0, 0.0, 0.5));
        assert!((curve.eval(0.4) - 0.5).abs() < 1e-6);
        assert!((curve.eval(0.8) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn curves_are_checked() {
        let one = |points: Vec<[f32; 2]>| vec![RawCurve { input: Input::Rpm, points }];
        assert!(check_curves(one(vec![[0.0, 0.0], [1.0, 1.0]])).is_ok());
        for bad in [
            vec![[0.0, 0.0]],
            vec![[0.5, 0.0], [0.5, 1.0]],
            vec![[0.0, 0.0], [1.0, 9.0]],
            vec![[0.0, -1.0], [1.0, 1.0]],
            vec![[0.0, f32::NAN], [1.0, 1.0]],
        ] {
            assert!(check_curves(one(bad)).is_err());
        }
        let five = (0..5).map(|_| RawCurve { input: Input::Load, points: vec![[0.0, 0.0], [1.0, 1.0]] }).collect();
        assert!(check_curves(five).is_err());
        assert!(check_pitch(Some(RawPitch { input: Input::SpeedMph, reference: 0.0 })).is_err());
    }
}
