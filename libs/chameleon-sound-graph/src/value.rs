//! Value modulators and fades: the numbers a sequence item plays with.

use crate::bytes::{f32_at, string_at, u16_at, u32_at};
use crate::curve::{Curve, Point};

/// Bytes of one modulator in a block.
pub(crate) const MODULATOR_SIZE: usize = 56;
/// Bytes of one curve point.
const POINT_SIZE: usize = 36;
/// Most points a curve is read with.
const MAX_POINTS: usize = 64;

/// A value that can follow a live value: `expression` names it (empty for none), `input` is the range of the live
/// value that the curve spans, `output` the range its result is scaled to, and `base` the constant the modulator gives
/// when it has neither an expression nor a curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Modulator {
    pub expression: String,
    pub input: (f32, f32),
    pub output: (f32, f32),
    pub base: f32,
    pub scale: f32,
    pub curve: Curve,
}

impl Default for Modulator {
    fn default() -> Self {
        Self {
            expression: String::new(),
            input: (0.0, 1.0),
            output: (0.0, 1.0),
            base: 1.0,
            scale: 1.0,
            curve: Curve::default(),
        }
    }
}

impl Modulator {
    /// Read the modulator at `at` of `block`.
    pub(crate) fn read(block: &[u8], at: usize) -> Option<Self> {
        let count = usize::from(u16_at(block, at + 52)?).min(MAX_POINTS);
        let begin = u32_at(block, at + 44)? as usize;
        let mut points = Vec::new();
        for n in 0..count {
            let p = begin.checked_add(n * POINT_SIZE)?;
            points.push(Point {
                x: f32_at(block, p + 12)?,
                y: f32_at(block, p + 16)?,
                left: [f32_at(block, p + 20)?, f32_at(block, p + 24)?],
                right: [f32_at(block, p + 28)?, f32_at(block, p + 32)?],
            });
        }
        Some(Self {
            expression: string_at(block, at + 12)?,
            scale: f32_at(block, at + 20)?,
            input: (f32_at(block, at + 28)?, f32_at(block, at + 24)?),
            output: (f32_at(block, at + 36)?, f32_at(block, at + 32)?),
            base: f32_at(block, at + 40)?,
            curve: Curve { points },
        })
    }

    /// Whether the modulator follows an input: it names a live value or has a curve.
    pub fn follows_input(&self) -> bool {
        !self.expression.is_empty() || !self.curve.is_empty()
    }

    /// The value for the live value `input`: the constant `base` when the modulator follows no input, else the input
    /// normalised over [`Modulator::input`], through the curve, scaled over [`Modulator::output`].
    pub fn eval(&self, input: f32) -> f32 {
        if !self.follows_input() {
            return self.base;
        }
        let span = self.input.1 - self.input.0;
        let t = if span.abs() <= f32::EPSILON { 0.0 } else { ((input - self.input.0) / span).clamp(0.0, 1.0) };
        let shaped = self.curve.eval(t);
        self.scale * (self.output.0 + (self.output.1 - self.output.0) * shaped)
    }
}

/// A fade at one end of an item: it spans `span` units of the live value and shapes the gain as `t ^ shape`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fade {
    pub shape: f32,
    pub span: f32,
}

impl Fade {
    pub(crate) fn read(block: &[u8], at: usize) -> Option<Self> {
        Some(Self { shape: f32_at(block, at + 12)?, span: f32_at(block, at + 16)? })
    }

    /// The gain `distance` units into the fade: 1 when there is no fade or it is over.
    pub fn gain(&self, distance: f32) -> f32 {
        if self.span <= f32::EPSILON {
            return 1.0;
        }
        let t = (distance / self.span).clamp(0.0, 1.0);
        t.powf(self.shape.max(0.01))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_modulator_with_no_input_is_its_base() {
        let value = Modulator { base: 0.6, ..Modulator::default() };
        assert_eq!(value.eval(123.0), 0.6);
    }

    #[test]
    fn an_expression_maps_its_input_range_to_its_output_range() {
        let value = Modulator {
            expression: "x.GetSpeedMph".into(),
            input: (50.0, 100.0),
            output: (0.0, 3.0),
            ..Modulator::default()
        };
        assert_eq!(value.eval(10.0), 0.0);
        assert!((value.eval(75.0) - 1.5).abs() < 1e-5);
        assert_eq!(value.eval(500.0), 3.0);
    }

    #[test]
    fn fades_ramp_with_their_shape_and_a_zero_span_is_a_hard_edge() {
        let equal_power = Fade { shape: 0.5, span: 100.0 };
        assert!((equal_power.gain(25.0) - 0.5).abs() < 1e-5);
        assert_eq!(equal_power.gain(400.0), 1.0);
        assert_eq!(Fade { shape: 1.0, span: 0.0 }.gain(0.0), 1.0);
    }
}
