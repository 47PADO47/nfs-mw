//! Complete appearance recipes; applied explicitly without storing stale preset labels.
use super::{TailTrailStyle, TrailShape};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrailPreset {
    EnhancedMw,
    LongExposure,
    Pursuit,
}
impl TrailPreset {
    pub fn apply(self, previous: TailTrailStyle) -> TailTrailStyle {
        let mut s =
            TailTrailStyle { enabled: previous.enabled, enhanced: true, tint: previous.tint, ..Default::default() };
        match self {
            Self::EnhancedMw => {
                s.width = 90;
                s.turbulence = 35;
                s.glow = 85;
            }
            Self::LongExposure => {
                s.shape = TrailShape::Ribbon;
                s.exposure = 170;
                s.length = 150;
                s.width = 55;
                s.brightness = 115;
                s.glow = 65;
                s.turbulence = 0;
                s.engagement = 35;
                s.lamp_bloom = 75;
            }
            Self::Pursuit => {
                s.exposure = 70;
                s.length = 75;
                s.width = 85;
                s.brightness = 130;
                s.lamp_glow = 130;
                s.lamp_bloom = 130;
                s.engagement = 160;
                s.turbulence = 50;
            }
        }
        s
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipes_are_valid_distinct_and_preserve_tint() {
        let old = TailTrailStyle { tint: [40, 90, 220], ..Default::default() };
        let recipes = [TrailPreset::EnhancedMw, TrailPreset::LongExposure, TrailPreset::Pursuit].map(|p| p.apply(old));
        for s in recipes {
            s.validate().unwrap();
            assert_eq!(s.tint, old.tint);
            assert!(s.enhanced);
        }
        assert_ne!(recipes[0], recipes[1]);
        assert_ne!(recipes[1], recipes[2]);
    }
    #[test]
    fn old_settings_get_new_defaults_and_round_trip() {
        let old: TailTrailStyle = toml::from_str("enhanced = true\nbrightness = 125").unwrap();
        assert_eq!(old.lamp_bloom, 100);
        assert_eq!(old.engagement, 100);
        assert!(old.follow_lamp);
        let encoded = toml::to_string(&old).unwrap();
        assert_eq!(toml::from_str::<TailTrailStyle>(&encoded).unwrap(), old);
        assert!(TailTrailStyle { lamp_bloom: 201, ..old }.validate().is_err());
    }
}
