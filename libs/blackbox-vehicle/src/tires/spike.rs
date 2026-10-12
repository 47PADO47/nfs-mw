//! The spike-strip contact test (spec: `docs/specs/ai-pursuit-roadblocks.md` section 6.2): does a tire, moving
//! during a step, overlap the strip's oriented box? The spec says the original's box routine is not in the
//! decompilation, so the sweep here is the intended behaviour, sampled at the start, the middle and the end of
//! the step rather than solved exactly.

use glam::Vec3;

/// An oriented box: its centre, three unit axes and half extents along them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedBox {
    pub centre: Vec3,
    pub axes: [Vec3; 3],
    pub half: Vec3,
}

impl OrientedBox {
    /// Half-width of the box's projection onto a unit `axis`.
    fn radius_on(&self, axis: Vec3) -> f32 {
        self.axes.iter().zip(self.half.to_array()).map(|(a, h)| a.dot(axis).abs() * h).sum()
    }

    /// The same box moved by `offset`.
    fn moved(&self, offset: Vec3) -> Self {
        Self { centre: self.centre + offset, ..*self }
    }
}

/// Separating-axis test between two oriented boxes (touching counts as overlapping).
pub fn boxes_overlap(a: &OrientedBox, b: &OrientedBox) -> bool {
    let delta = b.centre - a.centre;
    let candidates = a
        .axes
        .iter()
        .chain(b.axes.iter())
        .copied()
        .chain(a.axes.iter().flat_map(|x| b.axes.iter().map(move |y| x.cross(*y))));
    candidates.filter(|axis| axis.length_squared() > 1e-8).all(|axis| {
        let axis = axis.normalize();
        delta.dot(axis).abs() <= a.radius_on(axis) + b.radius_on(axis)
    })
}

/// Whether `tire` moving by `motion` during the step overlaps `strip`: sampled at the start, the middle and the end.
pub fn swept_overlap(strip: &OrientedBox, tire: &OrientedBox, motion: Vec3) -> bool {
    [0.0, 0.5, 1.0].into_iter().any(|t| boxes_overlap(strip, &tire.moved(motion * t)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_box(centre: Vec3) -> OrientedBox {
        OrientedBox { centre, axes: [Vec3::X, Vec3::Y, Vec3::Z], half: Vec3::splat(0.5) }
    }

    #[test]
    fn boxes_overlap_only_when_they_touch_or_intersect() {
        assert!(boxes_overlap(&unit_box(Vec3::ZERO), &unit_box(Vec3::new(0.9, 0.0, 0.0))));
        assert!(!boxes_overlap(&unit_box(Vec3::ZERO), &unit_box(Vec3::new(1.2, 0.0, 0.0))));
    }

    #[test]
    fn rotated_boxes_use_their_own_axes() {
        let strip =
            OrientedBox { centre: Vec3::ZERO, axes: [Vec3::X, Vec3::Y, Vec3::Z], half: Vec3::new(2.0, 0.1, 0.5) };
        // A thin box across the strip's length, turned 90 degrees about Z: it reaches the strip.
        let across = OrientedBox {
            centre: Vec3::new(0.0, 0.15, 0.0),
            axes: [Vec3::Y, Vec3::NEG_X, Vec3::Z],
            half: Vec3::new(0.2, 0.5, 0.5),
        };
        assert!(boxes_overlap(&strip, &across));
        let far = OrientedBox { centre: Vec3::new(0.0, 3.0, 0.0), ..across };
        assert!(!boxes_overlap(&strip, &far));
    }

    #[test]
    fn a_fast_tire_passing_through_the_strip_is_caught_by_the_sweep() {
        let strip = unit_box(Vec3::ZERO);
        let tire = unit_box(Vec3::new(-3.0, 0.0, 0.0));
        assert!(!boxes_overlap(&strip, &tire), "not touching at the start");
        // 4 m in one step: the tire ends beyond the strip, but passes through it.
        assert!(swept_overlap(&strip, &tire, Vec3::new(4.0, 0.0, 0.0)));
        assert!(!swept_overlap(&strip, &tire, Vec3::new(0.5, 0.0, 0.0)));
    }
}
