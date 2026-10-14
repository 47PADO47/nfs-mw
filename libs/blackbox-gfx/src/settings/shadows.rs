//! Dynamic directional shadow maps. Quality controls resolution, cascades and range.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ShadowQuality {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ShadowQuality {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Shadow map edge in texels, cascade count and maximum receiver distance in world units.
    pub const fn sampling(self) -> (usize, usize, f32) {
        match self {
            Self::Off => (1024, 1, 0.0),
            Self::Low => (1024, 2, 80.0),
            Self::Medium => (2048, 3, 160.0),
            Self::High => (2048, 4, 240.0),
        }
    }
}

impl std::str::FromStr for ShadowQuality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|q| q.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected off, low, medium or high, got {s:?}"))
    }
}

impl std::fmt::Display for ShadowQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
