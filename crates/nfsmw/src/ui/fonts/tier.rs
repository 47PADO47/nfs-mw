//! The resolution tier of the custom fonts: which set a render height uses. Rule: `docs/specs/custom-fonts.md`, section 3.

/// The render height (physical pixels) from which the High tier is used.
pub const HIGH_MIN_HEIGHT: u32 = 1200;
/// The render height (physical pixels) from which the Mid tier is used; below it, Low.
pub const MID_MIN_HEIGHT: u32 = 960;

/// One resolution tier. Higher tiers are drawn larger for higher render heights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    High,
    Mid,
    Low,
}

impl Tier {
    /// Every tier, from the highest.
    pub const ALL: [Tier; 3] = [Tier::High, Tier::Mid, Tier::Low];

    /// The position in [`Tier::ALL`]: 0 for High, 2 for Low.
    pub fn index(self) -> usize {
        match self {
            Tier::High => 0,
            Tier::Mid => 1,
            Tier::Low => 2,
        }
    }

    /// The tier's folder name, and its name in the config file and the console.
    pub fn name(self) -> &'static str {
        match self {
            Tier::High => "high",
            Tier::Mid => "mid",
            Tier::Low => "low",
        }
    }

    /// The tiers searched for a font when this tier is in use: this one, then each lower one down to Low. A higher
    /// tier's file is never used at a lower tier.
    pub fn chain(self) -> impl Iterator<Item = Tier> {
        Tier::ALL.into_iter().skip(self.index())
    }
}

/// The tier setting: chosen from the render height, or pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TierSetting {
    #[default]
    Auto,
    Pinned(Tier),
}

impl TierSetting {
    /// The config and console name: `auto`, `high`, `mid` or `low`.
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(TierSetting::Auto),
            "high" => Ok(TierSetting::Pinned(Tier::High)),
            "mid" => Ok(TierSetting::Pinned(Tier::Mid)),
            "low" => Ok(TierSetting::Pinned(Tier::Low)),
            _ => Err(format!("expected auto, high, mid or low, got {name:?}")),
        }
    }

    /// The name [`TierSetting::from_name`] reads back.
    pub fn name(self) -> &'static str {
        match self {
            TierSetting::Auto => "auto",
            TierSetting::Pinned(tier) => tier.name(),
        }
    }
}

/// The tier for a render height: High at or above [`HIGH_MIN_HEIGHT`], Mid at or above [`MID_MIN_HEIGHT`], else Low.
/// The boundary belongs to the upper tier.
pub fn by_height(height: u32) -> Tier {
    if height >= HIGH_MIN_HEIGHT {
        return Tier::High;
    }
    if height >= MID_MIN_HEIGHT {
        return Tier::Mid;
    }
    Tier::Low
}

/// The tier in use: the pinned one, or the one the render height selects.
pub fn select(setting: TierSetting, height: u32) -> Tier {
    match setting {
        TierSetting::Pinned(tier) => tier,
        TierSetting::Auto => by_height(height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_height_boundaries_belong_to_the_upper_tier() {
        assert_eq!(by_height(1199), Tier::Mid);
        assert_eq!(by_height(1200), Tier::High);
        assert_eq!(by_height(959), Tier::Low);
        assert_eq!(by_height(960), Tier::Mid);
        assert_eq!(by_height(0), Tier::Low);
    }

    #[test]
    fn a_pinned_tier_ignores_the_height() {
        assert_eq!(select(TierSetting::Pinned(Tier::Low), 2160), Tier::Low);
        assert_eq!(select(TierSetting::Auto, 2160), Tier::High);
        assert_eq!(select(TierSetting::Auto, 1080), Tier::Mid);
    }

    #[test]
    fn a_tier_only_falls_back_to_lower_tiers() {
        assert_eq!(Tier::High.chain().collect::<Vec<_>>(), [Tier::High, Tier::Mid, Tier::Low]);
        assert_eq!(Tier::Mid.chain().collect::<Vec<_>>(), [Tier::Mid, Tier::Low]);
        assert_eq!(Tier::Low.chain().collect::<Vec<_>>(), [Tier::Low]);
    }

    #[test]
    fn the_names_round_trip() {
        for name in ["auto", "high", "mid", "low"] {
            assert_eq!(TierSetting::from_name(name).unwrap().name(), name);
        }
        assert_eq!(TierSetting::from_name(" HIGH ").unwrap(), TierSetting::Pinned(Tier::High));
        assert!(TierSetting::from_name("ultra").is_err());
    }
}
