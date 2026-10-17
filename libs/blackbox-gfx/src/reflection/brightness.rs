//! The vehicle reflection brightness: a multiplier on the environment term of the car's glossy material. The original
//! has no such setting; 100 percent is the port's current look. Text is a percentage: `100`, `100%` or `150.5%`.

use std::str::FromStr;

/// Smallest percentage: the vehicles get no environment reflection.
pub const MIN_PERCENT: f32 = 0.0;
/// Largest percentage: the environment term is three times the material's own.
pub const MAX_PERCENT: f32 = 300.0;

/// The environment reflection multiplier on the vehicles, as a factor (1.0 is 100 percent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleBrightness(pub f32);

impl VehicleBrightness {
    /// The material's own environment term.
    pub const DEFAULT: Self = Self(1.0);

    /// The factor clamped into [`MIN_PERCENT`]..=[`MAX_PERCENT`] as a factor; a non-finite factor becomes the default.
    pub fn sanitized(self) -> Self {
        if !self.0.is_finite() {
            return Self::DEFAULT;
        }
        Self(self.0.clamp(MIN_PERCENT / 100.0, MAX_PERCENT / 100.0))
    }
}

impl Default for VehicleBrightness {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl FromStr for VehicleBrightness {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim();
        let bad = || format!("expected a percentage from {MIN_PERCENT:.0}% to {MAX_PERCENT:.0}%, got {s:?}");
        let number = text.strip_suffix('%').unwrap_or(text).trim();
        let percent: f32 = number.parse().map_err(|_| bad())?;
        if !(MIN_PERCENT..=MAX_PERCENT).contains(&percent) {
            return Err(bad());
        }
        Ok(Self(percent / 100.0))
    }
}

impl std::fmt::Display for VehicleBrightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", (self.0 * 100.0).round() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_material_term_at_one_hundred_percent() {
        assert_eq!(VehicleBrightness::default(), VehicleBrightness(1.0));
        assert_eq!(VehicleBrightness::default().to_string(), "100%");
    }

    #[test]
    fn percentages_parse_with_or_without_the_sign() {
        assert_eq!("150%".parse::<VehicleBrightness>().unwrap(), VehicleBrightness(1.5));
        assert_eq!("150".parse::<VehicleBrightness>().unwrap(), VehicleBrightness(1.5));
        assert_eq!(" 300 ".parse::<VehicleBrightness>().unwrap(), VehicleBrightness(3.0));
        assert_eq!("0%".parse::<VehicleBrightness>().unwrap(), VehicleBrightness(0.0));
        for text in ["", "%", "301", "-1%", "nan", "bright", "1.5x"] {
            assert!(text.parse::<VehicleBrightness>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn display_round_trips_whole_percentages() {
        for text in ["0%", "50%", "100%", "300%"] {
            let value: VehicleBrightness = text.parse().unwrap();
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn sanitizing_clamps_the_factor_and_replaces_a_non_finite_one() {
        assert_eq!(VehicleBrightness(9.0).sanitized(), VehicleBrightness(3.0));
        assert_eq!(VehicleBrightness(-2.0).sanitized(), VehicleBrightness(0.0));
        assert_eq!(VehicleBrightness(f32::NAN).sanitized(), VehicleBrightness::DEFAULT);
        assert_eq!(VehicleBrightness(0.8).sanitized(), VehicleBrightness(0.8));
    }
}
