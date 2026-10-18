//! Experimental screen-space contact occlusion. No temporal history or game-specific data.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AoQuality {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl AoQuality {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Spatial resolution divisor, horizon slices and steps per side of each slice.
    pub const fn sampling(self) -> (u32, u32, u32) {
        match self {
            Self::Off => (2, 0, 0),
            Self::Low => (2, 2, 3),
            Self::Medium => (2, 3, 4),
            Self::High => (1, 4, 6),
        }
    }
}

impl std::str::FromStr for AoQuality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|q| q.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected off, low, medium or high, got {s:?}"))
    }
}

impl std::fmt::Display for AoQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Independent AO estimators sharing geometry, filtering and ambient composition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AoMethod {
    #[default]
    Gtao,
    Ssao,
    XeGtao,
}

impl AoMethod {
    pub const ALL: [Self; 3] = [Self::Gtao, Self::Ssao, Self::XeGtao];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gtao => "gtao",
            Self::Ssao => "ssao",
            Self::XeGtao => "xegtao",
        }
    }
}
impl std::str::FromStr for AoMethod {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|m| m.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected gtao, ssao or xegtao, got {s:?}"))
    }
}
impl std::fmt::Display for AoMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AoSettings {
    pub quality: AoQuality,
    pub method: AoMethod,
    /// Ambient visibility strength from 0 to 1. Direct lighting and reflections are unchanged.
    pub intensity: f32,
    /// Contact radius in world units; bounded to avoid large distant silhouettes.
    pub radius: f32,
    /// Left half without AO, right half with AO, in the same frame.
    pub compare: bool,
    /// Depth-aware mip sampling for distant horizon taps; false retains the direct-depth reference.
    pub depth_mips: bool,
}

impl Default for AoSettings {
    fn default() -> Self {
        Self {
            quality: AoQuality::Off,
            method: AoMethod::Gtao,
            intensity: 0.75,
            radius: 1.25,
            compare: false,
            depth_mips: false,
        }
    }
}

impl AoSettings {
    pub fn sanitized(self) -> Self {
        let defaults = Self::default();
        Self {
            intensity: finite_clamp(self.intensity, 0.0, 1.0, defaults.intensity),
            radius: finite_clamp(self.radius, 0.25, 3.0, defaults.radius),
            ..self
        }
    }

    pub fn active(self) -> bool {
        self.quality != AoQuality::Off && self.intensity > 0.0
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if !value.is_finite() {
        return fallback;
    }
    value.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_and_zero_strength_do_no_work_and_bad_numbers_are_bounded() {
        assert!(!AoSettings::default().active());
        let s = AoSettings { quality: AoQuality::Medium, intensity: 0.0, ..Default::default() };
        assert!(!s.active());
        let s = AoSettings { intensity: f32::NAN, radius: f32::INFINITY, ..s }.sanitized();
        assert!(s.active());
        assert_eq!(s.intensity, 0.75);
        assert_eq!(s.radius, 1.25);
        assert_eq!(AoSettings { intensity: 50.0, radius: -1.0, ..s }.sanitized().radius, 0.25);
    }
}
