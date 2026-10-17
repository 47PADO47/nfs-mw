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

/// The sun's shadow settings for one frame. A scene that needs its own (the menu backdrop) sets them on its frame
/// (`FrameParams::shadow_tuning`); the world sets none and gets [`ShadowTuning::world`] from its quality setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowTuning {
    /// The map resolution, cascades and range.
    pub quality: ShadowQuality,
    /// Multiplies the sun's depth and normal bias. 1 keeps the renderer's values.
    pub bias: f32,
    /// Divides the map edge (see [`divided_edge`]). 1 keeps it.
    pub divider: usize,
}

impl ShadowTuning {
    /// The world's tuning: its quality setting, the renderer's bias and the full map edge.
    pub const fn world(quality: ShadowQuality) -> Self {
        Self { quality, bias: 1.0, divider: 1 }
    }
}

/// The smallest map a front-end divider leaves. A divider never shrinks a map below this, or below its own edge.
pub const MIN_DIVIDED_EDGE: usize = 512;

/// The edge of a shadow map cut by `divider` (1, 2 or 4): `edge` over the divider, never below
/// [`MIN_DIVIDED_EDGE`] and never above `edge`.
pub fn divided_edge(edge: usize, divider: usize) -> usize {
    if divider <= 1 {
        return edge;
    }
    (edge / divider).max(MIN_DIVIDED_EDGE.min(edge))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_divider_cuts_the_map_and_stops_at_the_floor() {
        let medium = ShadowQuality::Medium.sampling().0;
        assert_eq!(divided_edge(medium, 1), 2048);
        assert_eq!(divided_edge(medium, 2), 1024);
        assert_eq!(divided_edge(medium, 4), 512);
        assert_eq!(divided_edge(1024, 4), 512);
        assert_eq!(divided_edge(256, 4), 256, "a map under the floor keeps its own edge");
    }
}
