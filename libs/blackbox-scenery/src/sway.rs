//! Wind sway of flagged scenery: the small rotation the engine applies to a tree's or a boat's instance matrix each
//! time a player view draws it. Spec: `docs/specs/scenery-sway.md`.
//!
//! Matrices here use the engine's row-vector convention: rows are the local axes in world space.

use std::f32::consts::TAU;

/// Constants of the sway (the game's [`crate::layout::SceneryLayout::sway`]).
#[derive(Debug, Clone)]
pub struct SwayRules {
    /// `eSolid::Flags` bit of a solid that sways.
    pub solid_flag: u16,
    /// Instance flag of a copy drawn without a matrix, which never sways.
    pub unmatrixed_flag: u32,
    /// Clock rate in degrees per second.
    pub rate: f32,
    /// Largest sway angle in degrees.
    pub amplitude: f32,
    /// Multiplies the instance's world x to spread neighbours over the cycle.
    pub seed_scale: f32,
}

/// One view's sway inputs for a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwayFrame {
    /// The clock phase in degrees, `0..360`.
    pub phase: f32,
    /// The wind vector the view hands on, any length. A zero vector stands for the engine's `(1, 0, 0)`.
    pub axis: [f32; 3],
}

impl SwayRules {
    /// The clock phase after `seconds` of world animation time.
    pub fn phase(&self, seconds: f32) -> f32 {
        (seconds * self.rate).rem_euclid(360.0)
    }

    /// Whether a solid with `flags` sways.
    pub fn solid_sways(&self, flags: u16) -> bool {
        flags & self.solid_flag != 0
    }

    /// Whether an instance with `flags` can sway (it is drawn with a matrix).
    pub fn instance_sways(&self, flags: u32) -> bool {
        flags & self.unmatrixed_flag == 0
    }

    /// The sway rotation `S` of an instance whose world rotation rows are `rotation` and whose world x is `x`. The
    /// swayed instance matrix is `S * M`.
    pub fn rotation(&self, frame: &SwayFrame, rotation: [[f32; 3]; 3], x: f32) -> [[f32; 3]; 3] {
        let seed = (x * self.seed_scale) as i32 % 360;
        let angle = binary_angle(self.amplitude * binary_angle(seed as f32 + frame.phase).sin());
        let wind = normalize_or_x(frame.axis);
        // The inverse of a pure yaw: the engine negates the two off-diagonal xy terms rather than inverting.
        let mut inverse = rotation;
        inverse[0][1] = -inverse[0][1];
        inverse[1][0] = -inverse[1][0];
        let local = std::array::from_fn(|j| (0..3).map(|i| wind[i] * inverse[i][j]).sum::<f32>());
        multiply(axis_rotation(local, angle), z_rotation(angle))
    }
}

/// `degrees` through the engine's 16-bit binary angle, in radians.
fn binary_angle(degrees: f32) -> f32 {
    let units = (degrees * 65536.0) as i32 / 360;
    f32::from(units as u16) * TAU / 65536.0
}

fn normalize_or_x(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length == 0.0 || !length.is_finite() {
        return [1.0, 0.0, 0.0];
    }
    v.map(|c| c / length)
}

/// The engine's rotation by `angle` about `axis`, which is used as it is (not normalized).
fn axis_rotation([x, y, z]: [f32; 3], angle: f32) -> [[f32; 3]; 3] {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - z * s, t * x * z + y * s],
        [t * x * y + z * s, t * y * y + c, t * y * z - x * s],
        [t * x * z - y * s, t * y * z + x * s, t * z * z + c],
    ]
}

fn z_rotation(angle: f32) -> [[f32; 3]; 3] {
    let (s, c) = angle.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}

fn multiply(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MOST_WANTED;

    const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    fn yaw(degrees: f32) -> [[f32; 3]; 3] {
        z_rotation(degrees.to_radians())
    }

    fn close(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> bool {
        a.iter().flatten().zip(b.iter().flatten()).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    /// The rotation angle of `m` from its trace.
    fn angle_of(m: [[f32; 3]; 3]) -> f32 {
        ((m[0][0] + m[1][1] + m[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos()
    }

    #[test]
    fn the_clock_turns_at_the_rate_and_wraps() {
        let rules = &MOST_WANTED.sway;
        assert_eq!(rules.phase(2.0), 90.0);
        assert_eq!(rules.phase(8.0), 0.0);
        assert_eq!(rules.phase(9.0), 45.0);
    }

    #[test]
    fn zero_phase_at_the_origin_is_at_rest() {
        let frame = SwayFrame { phase: 0.0, axis: [0.3, 0.4, 0.0] };
        assert!(close(MOST_WANTED.sway.rotation(&frame, IDENTITY, 0.0), IDENTITY));
    }

    #[test]
    fn the_angle_peaks_at_the_amplitude() {
        let rules = &MOST_WANTED.sway;
        let frame = SwayFrame { phase: 90.0, axis: [0.0, 0.0, 0.0] };
        let peak = binary_angle(rules.amplitude);
        let s = rules.rotation(&frame, IDENTITY, 0.0);
        // Two turns of the same angle about x and then z.
        let expected = multiply(axis_rotation([1.0, 0.0, 0.0], peak), z_rotation(peak));
        assert!(close(s, expected));
        for phase in (0..360).step_by(5) {
            let s = rules.rotation(&SwayFrame { phase: phase as f32, axis: [0.0, 1.0, 0.0] }, IDENTITY, 0.0);
            assert!(angle_of(s) <= 2.0 * rules.amplitude.to_radians() + 1e-4);
        }
    }

    #[test]
    fn neighbours_are_spread_by_their_x() {
        let rules = &MOST_WANTED.sway;
        let frame = SwayFrame { phase: 0.0, axis: [1.0, 0.0, 0.0] };
        // x = 1.5 seeds 90 degrees (the peak), x = -1.5 seeds -90 (the trough): opposite turns about x.
        let peak = rules.rotation(&frame, IDENTITY, 1.5);
        let trough = rules.rotation(&frame, IDENTITY, -1.5);
        assert!(peak[1][2] < 0.0 && trough[1][2] > 0.0, "{peak:?} {trough:?}");
        // 6 units apart is 360 degrees of seed: the same pose.
        assert!(close(rules.rotation(&frame, IDENTITY, 2.0), rules.rotation(&frame, IDENTITY, 8.0)));
    }

    #[test]
    fn a_yawed_instance_gets_a_proper_rotation() {
        let rules = &MOST_WANTED.sway;
        for (degrees, x) in [(30.0, 12.0), (135.0, -40.0), (270.0, 3.3)] {
            let s = rules.rotation(&SwayFrame { phase: 77.0, axis: [0.6, -0.8, 0.0] }, yaw(degrees), x);
            let t: [[f32; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| s[j][i]));
            assert!(close(multiply(s, t), IDENTITY), "orthonormal at {degrees}");
            let det = s[0][0] * (s[1][1] * s[2][2] - s[1][2] * s[2][1])
                - s[0][1] * (s[1][0] * s[2][2] - s[1][2] * s[2][0])
                + s[0][2] * (s[1][0] * s[2][1] - s[1][1] * s[2][0]);
            assert!((det - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn flags_choose_what_sways() {
        let rules = &MOST_WANTED.sway;
        assert!(rules.solid_sways(0x80) && rules.solid_sways(0xC0) && !rules.solid_sways(0x40));
        assert!(rules.instance_sways(0x12) && !rules.instance_sways(0x212));
    }
}
