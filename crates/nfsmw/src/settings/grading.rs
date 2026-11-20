//! The strength of the colour grading of a mod's variant (`docs/specs/colour-grading.md`, "Settings"): how far the frame
//! goes from its own colour (0) to the graded colour (1). The grading itself is chosen with the `grading_variant` option a
//! mod adds values to; this number scales whichever one is chosen.

/// The grade as the mod made it.
pub const DEFAULT_STRENGTH: f32 = 1.0;
/// The range the settings accept.
pub const STRENGTH_RANGE: (f32, f32) = (0.0, 1.0);

/// `strength` within the range; a number that is not finite is the default.
pub fn clamp_strength(strength: f32) -> f32 {
    match strength.is_finite() {
        true => strength.clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1),
        false => DEFAULT_STRENGTH,
    }
}

/// A strength typed as text (`0.5`), within the range the settings accept.
pub fn parse_strength(text: &str) -> Result<f32, String> {
    let (lo, hi) = STRENGTH_RANGE;
    let n: f32 = text.trim().parse().map_err(|_| format!("expected a number from {lo} to {hi}, got {text:?}"))?;
    match (lo..=hi).contains(&n) {
        true => Ok(n),
        false => Err(format!("expected a number from {lo} to {hi}, got {text:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strength_is_kept_in_its_range() {
        assert_eq!(clamp_strength(0.4), 0.4);
        assert_eq!((clamp_strength(-1.0), clamp_strength(7.0)), (0.0, 1.0));
        assert_eq!(clamp_strength(f32::NAN), DEFAULT_STRENGTH);
    }

    #[test]
    fn a_typed_strength_must_be_a_number_in_range() {
        assert_eq!(parse_strength(" 0.5 "), Ok(0.5));
        assert!(parse_strength("1.5").is_err() && parse_strength("high").is_err() && parse_strength("-0.1").is_err());
    }
}
