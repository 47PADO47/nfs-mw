//! The high-speed radial blur's switch and strength. The mod `NFSMWMotionBlurController` exposes the same two knobs
//! for the original game; here they scale the rewrite's own blur curve (`docs/specs/speed-effects.md`, section 4b).

use blackbox_speedfx::scale_blur;

use super::Settings;

/// The blur is on by default, as in the original.
pub const DEFAULT_MOTION_BLUR: bool = true;
/// The strength multiplier: 1 is the curve as it is.
pub const DEFAULT_MOTION_BLUR_STRENGTH: f32 = 1.0;
/// The strength range the settings accept: 0 draws no blur, 2 doubles the curve (clamped to the full blur).
pub const MOTION_BLUR_STRENGTH_RANGE: (f32, f32) = (0.0, 2.0);

/// The blur the world draws, as the settings set it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionBlur {
    /// Whether the high-speed radial blur is drawn at all.
    pub enabled: bool,
    /// The multiplier on the curve's blur, 0 to 2.
    pub strength: f32,
}

impl Default for MotionBlur {
    fn default() -> Self {
        Self { enabled: DEFAULT_MOTION_BLUR, strength: DEFAULT_MOTION_BLUR_STRENGTH }
    }
}

/// A strength typed as text (`0.5`), within the range the settings accept.
pub fn parse_strength(text: &str) -> Result<f32, String> {
    let (lo, hi) = MOTION_BLUR_STRENGTH_RANGE;
    let n: f32 = text.trim().parse().map_err(|_| format!("expected a number from {lo} to {hi}, got {text:?}"))?;
    match (lo..=hi).contains(&n) {
        true => Ok(n),
        false => Err(format!("expected a number from {lo} to {hi}, got {text:?}")),
    }
}

impl From<&Settings> for MotionBlur {
    fn from(s: &Settings) -> Self {
        Self { enabled: s.motion_blur, strength: s.motion_blur_strength }
    }
}

impl MotionBlur {
    /// The blur to draw this frame, from the curve's blur (0 to 1) for the car's speed and nitrous.
    pub fn radial_blur(&self, curve_blur: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        scale_blur(curve_blur, self.strength)
    }
}
