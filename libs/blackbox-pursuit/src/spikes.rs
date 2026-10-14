//! Spike strips on the road as geometry, and the tire test (spec `docs/specs/ai-pursuit-roadblocks.md` sections 2.5 and
//! 6.2). Pure: the caller gives the strips and the wheels' patches on the road plane (physics x and z). The strip's size
//! and the tire box are design values **[unconfirmed]**: the prop's collision box is not read yet.

use glam::Vec2;

/// Half the strip's length across the road, metres (unconfirmed).
pub const HALF_ACROSS: f32 = 2.0;
/// Half the strip's depth along its facing, metres (unconfirmed).
pub const HALF_ALONG: f32 = 0.5;

/// A spike strip on the road: its centre (physics x, z) and its facing (radians in the x-z plane). Its long side lies
/// across the facing, so a strip faced against the road lies across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub position: Vec2,
    pub heading: f32,
}

/// One wheel as a strip sees it: the centre of its contact patch on the road plane, the tire radius, and whether the
/// tire touches the road.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub position: Vec2,
    pub radius: f32,
    pub on_ground: bool,
}

/// Whether the tire's patch overlaps the strip. The patch is taken as a box of the tire radius around its centre, so the
/// strip is inflated by the radius on both axes.
pub fn overlaps(strip: &Strip, patch: &Patch) -> bool {
    let facing = Vec2::new(strip.heading.cos(), strip.heading.sin());
    let across = Vec2::new(-facing.y, facing.x);
    let offset = patch.position - strip.position;
    offset.dot(across).abs() <= HALF_ACROSS + patch.radius && offset.dot(facing).abs() <= HALF_ALONG + patch.radius
}

/// The wheels that run over a strip this step, in wheel order: grounded, and over any strip. A wheel in the air does
/// not puncture.
pub fn punctured(strips: &[Strip], wheels: &[Patch; 4]) -> [bool; 4] {
    std::array::from_fn(|i| {
        let patch = &wheels[i];
        patch.on_ground && strips.iter().any(|strip| overlaps(strip, patch))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip() -> Strip {
        Strip { position: Vec2::ZERO, heading: 0.0 }
    }

    fn patch(x: f32, z: f32, on_ground: bool) -> Patch {
        Patch { position: Vec2::new(x, z), radius: 0.3, on_ground }
    }

    #[test]
    fn a_strip_lies_across_its_facing() {
        // Facing +x: the strip is long along z.
        assert!(overlaps(&strip(), &patch(0.0, 1.5, true)), "across the strip, within its half length");
        assert!(!overlaps(&strip(), &patch(0.0, 3.0, true)), "beyond the half length across");
        assert!(!overlaps(&strip(), &patch(1.5, 0.0, true)), "in front of the strip, past its depth");
    }

    #[test]
    fn a_grounded_wheel_over_a_strip_punctures() {
        let wheels = [patch(0.0, 1.0, true), patch(0.0, -1.0, false), patch(-5.0, 0.0, true), patch(5.0, 0.0, true)];
        assert_eq!(punctured(&[strip()], &wheels), [true, false, false, false]);
    }

    #[test]
    fn no_strips_puncture_nothing() {
        let wheels = [patch(0.0, 0.0, true); 4];
        assert_eq!(punctured(&[], &wheels), [false; 4]);
    }
}
