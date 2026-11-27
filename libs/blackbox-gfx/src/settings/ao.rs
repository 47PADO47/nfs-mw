//! Independent spatial and temporal screen-space ambient occlusion settings.

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
    /// Half-resolution XeGTAO with independent motion-reprojected history.
    TemporalXeGtao,
}

impl AoMethod {
    pub const ALL: [Self; 4] = [Self::Gtao, Self::Ssao, Self::XeGtao, Self::TemporalXeGtao];
    pub const fn is_xe(self) -> bool {
        matches!(self, Self::XeGtao | Self::TemporalXeGtao)
    }
    pub const fn temporal(self) -> bool {
        matches!(self, Self::TemporalXeGtao)
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gtao => "gtao",
            Self::Ssao => "ssao",
            Self::XeGtao => "xegtao",
            Self::TemporalXeGtao => "xegtao-temporal",
        }
    }
}
impl std::str::FromStr for AoMethod {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|m| m.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected gtao, ssao, xegtao or xegtao-temporal, got {s:?}"))
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
    /// XeGTAO thin occluder compensation, 0 to 1: a horizon tap's offset along the view direction counts this much
    /// more towards the falloff distance, so an occluder well in front of the receiver (a duct under a ceiling, a
    /// post before a wall) stops darkening the surface behind it. 0 is the reference default.
    pub thin_occluders: f32,
    /// A shading normal whose cosine to the surface's depth normal is below this gives way to the depth normal, so a
    /// smoothed low-poly face does not occlude itself. 0 always keeps the shading normal. The default (about 70 degrees)
    /// catches the badly smoothed boxes of the original models and leaves the noisier depth normals of distant ground alone.
    pub normal_guard: f32,
    /// Resolve the direction of ambient visibility alongside scalar XeGTAO.
    pub bent_normals: bool,
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
            thin_occluders: 0.5,
            normal_guard: 0.35,
            bent_normals: false,
        }
    }
}

impl AoSettings {
    pub fn sanitized(self) -> Self {
        let defaults = Self::default();
        Self {
            intensity: finite_clamp(self.intensity, 0.0, 1.0, defaults.intensity),
            radius: finite_clamp(self.radius, 0.25, 3.0, defaults.radius),
            thin_occluders: finite_clamp(self.thin_occluders, 0.0, 1.0, defaults.thin_occluders),
            normal_guard: finite_clamp(self.normal_guard, 0.0, 1.0, defaults.normal_guard),
            ..self
        }
    }

    pub fn active(self) -> bool {
        self.quality != AoQuality::Off && self.intensity > 0.0
    }

    /// Directional visibility is available from both XeGTAO methods.
    pub fn directional(self) -> bool {
        self.bent_normals && self.active() && self.method.is_xe()
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
        assert_eq!(AoSettings { thin_occluders: f32::NAN, ..s }.sanitized().thin_occluders, 0.5);
        assert_eq!(AoSettings { thin_occluders: 4.0, ..s }.sanitized().thin_occluders, 1.0);
    }
}
