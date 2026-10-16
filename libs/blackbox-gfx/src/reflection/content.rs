//! What the car cube capture adds to the scenery. The rear-view mirror already draws traffic; the car cube does not by
//! default. `restored` adds traffic to the cube. Other restored classes (lights, flares, particles) are not part of this
//! value; see docs/specs/hd-reflections.md, section 17.

/// The extra content of the car cube.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReflectionContent {
    /// The port's current cube: scenery and props only.
    #[default]
    Original,
    /// The cube also draws traffic cars, as the mirror does.
    Restored,
}

impl ReflectionContent {
    pub const ALL: [Self; 2] = [Self::Original, Self::Restored];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Restored => "restored",
        }
    }

    /// Whether the car cube draws the traffic cars.
    pub const fn includes_traffic(self) -> bool {
        match self {
            Self::Original => false,
            Self::Restored => true,
        }
    }
}

impl std::str::FromStr for ReflectionContent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|c| c.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected original or restored, got {s:?}"))
    }
}

impl std::fmt::Display for ReflectionContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_adds_nothing_and_restored_adds_traffic() {
        assert_eq!(ReflectionContent::default(), ReflectionContent::Original);
        assert!(!ReflectionContent::Original.includes_traffic());
        assert!(ReflectionContent::Restored.includes_traffic());
    }

    #[test]
    fn content_text_round_trips_and_rejects_other_names() {
        for content in ReflectionContent::ALL {
            assert_eq!(content.to_string().parse::<ReflectionContent>().unwrap(), content);
        }
        assert_eq!(" Restored ".parse::<ReflectionContent>().unwrap(), ReflectionContent::Restored);
        for text in ["", "traffic", "on", "all"] {
            assert!(text.parse::<ReflectionContent>().is_err(), "{text:?}");
        }
    }
}
