//! The scenery detail rule of the car cube. The original's reflection views use a fixed 32 px threshold; the port's
//! cube follows its own face size by default. `view` uses the main view's rule for the cube instead.

/// Which screen height the car cube's scenery detail is chosen for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReflectionLod {
    /// The cube's own face size, so the detail threshold follows the capture (the port's current behaviour).
    #[default]
    Capture,
    /// The main view's rule, as if the cube were the player's view.
    View,
}

impl ReflectionLod {
    pub const ALL: [Self; 2] = [Self::Capture, Self::View];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::View => "view",
        }
    }

    /// Whether the cube takes its detail from the main view's rule rather than from its own face size.
    pub const fn uses_main_view(self) -> bool {
        match self {
            Self::Capture => false,
            Self::View => true,
        }
    }
}

impl std::str::FromStr for ReflectionLod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|l| l.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected capture or view, got {s:?}"))
    }
}

impl std::fmt::Display for ReflectionLod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_follows_the_capture_and_the_view_option_follows_the_main_view() {
        assert_eq!(ReflectionLod::default(), ReflectionLod::Capture);
        assert!(!ReflectionLod::Capture.uses_main_view());
        assert!(ReflectionLod::View.uses_main_view());
    }

    #[test]
    fn lod_text_round_trips_and_rejects_other_names() {
        for lod in ReflectionLod::ALL {
            assert_eq!(lod.to_string().parse::<ReflectionLod>().unwrap(), lod);
        }
        assert_eq!("View".parse::<ReflectionLod>().unwrap(), ReflectionLod::View);
        for text in ["", "original", "high", "480"] {
            assert!(text.parse::<ReflectionLod>().is_err(), "{text:?}");
        }
    }
}
