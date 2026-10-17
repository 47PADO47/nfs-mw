//! The five health tiers and the fill colour of each. Spec section 4 (the thresholds and colours are port choices).

/// Lower bounds of the four upper tiers, inclusive (`TIER_THRESHOLDS`). Below the last one is red.
pub const THRESHOLDS: [f32; 4] = [0.75, 0.50, 0.25, 0.10];

/// The tier of a bar, from full health down to red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Green,
    LightGreen,
    Yellow,
    Orange,
    Red,
}

/// The tier of `health` (a fraction, 0 to 1). Each lower bound starts its own tier; 0 and anything below 0.10 is red.
pub fn tier_of(health: f32) -> Tier {
    if health >= THRESHOLDS[0] {
        return Tier::Green;
    }
    if health >= THRESHOLDS[1] {
        return Tier::LightGreen;
    }
    if health >= THRESHOLDS[2] {
        return Tier::Yellow;
    }
    if health >= THRESHOLDS[3] {
        return Tier::Orange;
    }
    Tier::Red
}

/// The fill colour of a tier, as sRGB bytes (see [`super::rgba`] for the linear form).
pub fn tier_rgb(tier: Tier) -> [u8; 3] {
    match tier {
        Tier::Green => [0x40, 0xd0, 0x40],
        Tier::LightGreen => [0x9a, 0xe6, 0x5a],
        Tier::Yellow => [0xf0, 0xd2, 0x3c],
        Tier::Orange => [0xf0, 0x8c, 0x28],
        Tier::Red => [0xe0, 0x28, 0x28],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_and_three_quarter_health_are_green() {
        assert_eq!(tier_of(1.0), Tier::Green);
        assert_eq!(tier_of(0.75), Tier::Green);
    }

    #[test]
    fn just_under_three_quarters_is_light_green() {
        assert_eq!(tier_of(0.7499), Tier::LightGreen);
        assert_eq!(tier_of(0.5), Tier::LightGreen);
    }

    #[test]
    fn each_lower_bound_starts_its_tier() {
        assert_eq!(tier_of(0.4999), Tier::Yellow);
        assert_eq!(tier_of(0.25), Tier::Yellow);
        assert_eq!(tier_of(0.0999), Tier::Red);
        assert_eq!(tier_of(0.10), Tier::Orange);
    }

    #[test]
    fn zero_health_is_red() {
        assert_eq!(tier_of(0.0), Tier::Red);
    }
}
