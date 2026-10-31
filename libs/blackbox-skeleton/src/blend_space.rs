//! Blend spaces: clips placed on a grid of two parameters, mixed by bilinear weights.

use serde::{Deserialize, Serialize};

/// One axis of the grid: the parameter range split into `divisions` equal cells (`divisions + 1` positions). With
/// `divisions == 0` the axis has a single position and the parameter is ignored.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    pub min: f32,
    pub max: f32,
    pub divisions: usize,
}

impl Axis {
    /// The two grid positions around `value` and the fraction between them. Values outside the range are clamped.
    #[must_use]
    pub fn locate(&self, value: f32) -> (usize, usize, f32) {
        if self.divisions == 0 || self.max <= self.min {
            return (0, 0, 0.0);
        }
        let position = ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0) * self.divisions as f32;
        let low = (position.floor() as usize).min(self.divisions - 1);
        (low, low + 1, position - low as f32)
    }
}

/// A clip at a grid position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceSample {
    pub x: usize,
    pub y: usize,
    /// Index into the clip list.
    pub clip: usize,
}

/// The grid and its samples. Positions without a sample are skipped and the remaining weights renormalised.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlendSpace {
    pub x: Axis,
    pub y: Axis,
    pub samples: Vec<SpaceSample>,
}

impl BlendSpace {
    /// The samples to mix at `(x, y)`, as `(index into self.samples, weight)`; the weights add up to 1, or the list
    /// is empty when no corner of the cell holds a sample.
    #[must_use]
    pub fn weights(&self, x: f32, y: f32) -> Vec<(usize, f32)> {
        let (x0, x1, fx) = self.x.locate(x);
        let (y0, y1, fy) = self.y.locate(y);
        let corners = [
            (x0, y0, (1.0 - fx) * (1.0 - fy)),
            (x1, y0, fx * (1.0 - fy)),
            (x0, y1, (1.0 - fx) * fy),
            (x1, y1, fx * fy),
        ];
        let mut out: Vec<(usize, f32)> = Vec::with_capacity(4);
        for (cx, cy, weight) in corners {
            if weight <= 0.0 {
                continue;
            }
            let Some(index) = self.samples.iter().position(|s| s.x == cx && s.y == cy) else { continue };
            match out.iter_mut().find(|(i, _)| *i == index) {
                Some(entry) => entry.1 += weight,
                None => out.push((index, weight)),
            }
        }
        let total: f32 = out.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            return Vec::new();
        }
        out.iter().map(|&(i, w)| (i, w / total)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space() -> BlendSpace {
        let axis = Axis { min: 0.0, max: 1.0, divisions: 1 };
        BlendSpace {
            x: axis,
            y: axis,
            samples: vec![
                SpaceSample { x: 0, y: 0, clip: 10 },
                SpaceSample { x: 1, y: 0, clip: 11 },
                SpaceSample { x: 0, y: 1, clip: 12 },
                SpaceSample { x: 1, y: 1, clip: 13 },
            ],
        }
    }

    #[test]
    fn bilinear_weights_add_to_one() {
        let w = space().weights(0.25, 0.5);
        let total: f32 = w.iter().map(|(_, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-6);
        let of = |i: usize| w.iter().find(|(s, _)| *s == i).map_or(0.0, |(_, w)| *w);
        assert!((of(0) - 0.375).abs() < 1e-6);
        assert!((of(3) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn exact_grid_point_selects_one_sample_and_range_is_clamped() {
        let w = space().weights(5.0, -5.0);
        assert_eq!(w, vec![(1, 1.0)]);
    }

    #[test]
    fn missing_corner_is_renormalised_and_flat_axis_is_ignored() {
        let mut s = space();
        s.samples.pop();
        let w = s.weights(1.0, 1.0);
        assert!(w.is_empty());
        let one_d = BlendSpace {
            x: Axis { min: 0.0, max: 2.0, divisions: 2 },
            y: Axis { min: 0.0, max: 0.0, divisions: 0 },
            samples: (0..3).map(|i| SpaceSample { x: i, y: 0, clip: i }).collect(),
        };
        let w = one_d.weights(1.5, 99.0);
        assert_eq!(w.len(), 2);
        assert!((w[0].1 - 0.5).abs() < 1e-6);
    }
}
