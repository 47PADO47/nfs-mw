//! Spike strips on the road as geometry, and the tire test (spec `docs/specs/ai-pursuit-roadblocks.md` sections 2.5 and
//! 6.2). Pure: the caller gives the strips and the wheels on the road plane (physics x and z) with their heights.
//!
//! The strip is the prop's collision box and the tire is the original's tire box: half 0.15 m along the axle and the
//! tire radius along the other two axes, swept by the car's motion during the step. The original's box routine is not in
//! the sources read, so the overlap is a separating-axis test of the two boxes in the road plane plus a height test,
//! sampled along the sweep. **[verified]** box size (install prop bounds), **[decomp]** tire box, **[inferred]** overlap.

use glam::Vec2;

/// Half the strip's length across the road, metres: the installed `XO_SpikeBelt_1b_DW_00` collision box is 6.53 m wide.
/// **[verified]** (prop bounds of the install; half extents 0.378, 3.265, 0.254 in model axes).
pub const HALF_ACROSS: f32 = 3.265;
/// Half the strip's depth along its facing, metres. **[verified]**
pub const HALF_ALONG: f32 = 0.378;
/// Half the strip box's height, metres. **[verified]**
pub const HALF_HEIGHT: f32 = 0.254;
/// How far the box centre sits above the prop's origin (the road surface under it), metres. **[verified]**
pub const CENTRE_LIFT: f32 = 0.013;
/// Half the tire box's width along the axle, metres (`SpikeStrip::OnCollide`). **[decomp]**
pub const TIRE_HALF_WIDTH: f32 = 0.15;
/// The sweep is sampled at least this often along the tire's path, metres.
const SWEEP_STEP: f32 = 0.05;

/// A spike strip on the road: its centre (physics x, z) and its facing (radians in the x-z plane). Its long side lies
/// across the facing, so a strip faced against the road lies across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub position: Vec2,
    /// Height of the road surface under the strip's origin, in the caller's vertical axis.
    pub elevation: f32,
    pub heading: f32,
    /// Half extents of the collision box: along the facing, across it, and up.
    pub half: [f32; 3],
}

impl Strip {
    /// A strip with the installed prop's collision box.
    pub fn new(position: Vec2, elevation: f32, heading: f32) -> Self {
        Self { position, elevation, heading, half: [HALF_ALONG, HALF_ACROSS, HALF_HEIGHT] }
    }

    /// The same strip with another collision box (`along`, `across`, `up` half extents).
    pub fn with_half(mut self, half: [f32; 3]) -> Self {
        self.half = half;
        self
    }

    /// The unit vectors along the facing and across it, on the road plane.
    pub fn axes(&self) -> (Vec2, Vec2) {
        let facing = Vec2::new(self.heading.cos(), self.heading.sin());
        (facing, Vec2::new(-facing.y, facing.x))
    }
}

/// One wheel as a strip sees it: its centre on the road plane and its height, the tire radius, the horizontal direction
/// of its axle (zero when unknown: the tire is then a square of its radius), and whether the tire touches the road.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub position: Vec2,
    /// The wheel centre's height.
    pub elevation: f32,
    pub radius: f32,
    pub axle: Vec2,
    pub on_ground: bool,
}

/// Whether the tire box overlaps the strip box.
pub fn overlaps(strip: &Strip, patch: &Patch) -> bool {
    let low = strip.elevation + CENTRE_LIFT - strip.half[2];
    let high = strip.elevation + CENTRE_LIFT + strip.half[2];
    if patch.elevation + patch.radius < low || patch.elevation - patch.radius > high {
        return false;
    }
    let (along, across) = strip.axes();
    let axle = patch.axle.try_normalize();
    let (tire_u, tire_v, half_u, half_v) = match axle {
        Some(axle) => (axle, Vec2::new(-axle.y, axle.x), TIRE_HALF_WIDTH, patch.radius),
        None => (along, across, patch.radius, patch.radius),
    };
    let delta = patch.position - strip.position;
    let strip_axes = [(along, strip.half[0]), (across, strip.half[1])];
    let tire_axes = [(tire_u, half_u), (tire_v, half_v)];
    // Separating axes of two rectangles: the edge normals of each.
    strip_axes.iter().chain(tire_axes.iter()).all(|&(axis, _)| {
        let reach = |axes: &[(Vec2, f32); 2]| axes.iter().map(|&(a, h)| a.dot(axis).abs() * h).sum::<f32>();
        delta.dot(axis).abs() <= reach(&strip_axes) + reach(&tire_axes)
    })
}

/// The wheels that run over a strip this step, in wheel order: grounded, and over any strip. A wheel in the air does
/// not puncture.
pub fn punctured(strips: &[Strip], wheels: &[Patch; 4]) -> [bool; 4] {
    std::array::from_fn(|i| {
        let patch = &wheels[i];
        patch.on_ground && strips.iter().any(|strip| overlaps(strip, patch))
    })
}

/// As [`punctured`], for a tire that moved from `from` to `to` during the step: the tire box is swept along its path, so
/// a fast tire cannot skip over the strip between two steps. Both ends must be grounded for the sweep; the end alone
/// counts when the start was in the air.
pub fn punctured_sweep(strips: &[Strip], from: &[Patch; 4], to: &[Patch; 4]) -> [bool; 4] {
    std::array::from_fn(|i| {
        let (a, b) = (from[i], to[i]);
        b.on_ground && strips.iter().any(|strip| overlaps(strip, &b) || (a.on_ground && swept(strip, a, b)))
    })
}

fn swept(strip: &Strip, a: Patch, b: Patch) -> bool {
    let length = a.position.distance(b.position);
    let steps = ((length / SWEEP_STEP).ceil() as usize).clamp(1, 400);
    (0..steps).any(|n| {
        let t = n as f32 / steps as f32;
        let mid = Patch {
            position: a.position.lerp(b.position, t),
            elevation: a.elevation + (b.elevation - a.elevation) * t,
            ..b
        };
        overlaps(strip, &mid)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip() -> Strip {
        Strip::new(Vec2::ZERO, 0.0, 0.0)
    }

    fn patch(x: f32, z: f32, on_ground: bool) -> Patch {
        Patch { position: Vec2::new(x, z), elevation: 0.3, radius: 0.3, axle: Vec2::ZERO, on_ground }
    }

    #[test]
    fn the_strip_is_the_installed_props_box() {
        assert_eq!(strip().half, [0.378, 3.265, 0.254]);
    }

    #[test]
    fn a_strip_lies_across_its_facing() {
        // Facing +x: the strip is long along z.
        assert!(overlaps(&strip(), &patch(0.0, 3.0, true)), "across the strip, within its half length");
        assert!(!overlaps(&strip(), &patch(0.0, 4.0, true)), "beyond the half length across");
        assert!(!overlaps(&strip(), &patch(1.5, 0.0, true)), "in front of the strip, past its depth");
    }

    #[test]
    fn the_tire_box_is_narrow_along_its_axle() {
        // A tire whose axle points along the strip's long side is only 0.15 m wide in that direction, 0.3 m deep.
        let mut tire = patch(0.0, 3.265 + 0.2, true);
        tire.axle = Vec2::new(0.0, 1.0);
        assert!(!overlaps(&strip(), &tire), "0.2 m past the end is outside a 0.15 m half width");
        tire.position.y = 3.265 + 0.1;
        assert!(overlaps(&strip(), &tire), "0.1 m past the end is inside it");
        // The same tire turned a quarter: the axle is along the car's way, so the tire is 0.3 m wide across the strip.
        tire.axle = Vec2::new(1.0, 0.0);
        tire.position.y = 3.265 + 0.2;
        assert!(overlaps(&strip(), &tire));
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

    #[test]
    fn tires_on_another_road_deck_are_not_punctured() {
        let mut wheels = [patch(0.0, 0.0, true); 4];
        wheels[1].elevation = 6.0;
        wheels[2].elevation = -6.0;
        wheels[3].on_ground = false;
        assert_eq!(punctured(&[strip()], &wheels), [true, false, false, false]);
    }

    #[test]
    fn a_strip_on_a_slope_follows_its_own_elevation() {
        let high = Strip::new(Vec2::ZERO, 10.0, 0.0);
        let mut wheel = patch(0.0, 0.0, true);
        assert!(!overlaps(&high, &wheel), "a wheel on the road 10 m below");
        wheel.elevation = 10.3;
        assert!(overlaps(&high, &wheel));
    }

    #[test]
    fn fast_tires_crossing_the_whole_strip_in_one_step_still_puncture() {
        let from = [patch(-2.0, 0.0, true); 4];
        let mut to = [patch(2.0, 0.0, true); 4];
        to[1].position.y = 30.0;
        to[2].on_ground = false;
        let mut elevated = from;
        elevated[3].elevation = 6.0;
        to[3].elevation = 6.0;
        assert_eq!(punctured_sweep(&[strip()], &elevated, &to), [true, false, false, false]);
    }

    #[test]
    fn a_sweep_that_starts_in_the_air_only_counts_its_end() {
        let mut from = [patch(-2.0, 0.0, true); 4];
        from[0].on_ground = false;
        let to = [patch(2.0, 0.0, true); 4];
        assert_eq!(punctured_sweep(&[strip()], &from, &to), [false, true, true, true]);
    }
}
