//! Display output is independent of the scene's artistic style and post effects.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HdrMode {
    Off,
    #[default]
    Auto,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplaySettings {
    pub hdr: HdrMode,
    /// Zero uses the display's advisory peak, falling back to 1000 nits.
    pub peak_nits: f32,
    pub scene_white_nits: f32,
    pub ui_white_nits: f32,
}
impl Default for DisplaySettings {
    fn default() -> Self {
        Self { hdr: HdrMode::Auto, peak_nits: 0.0, scene_white_nits: 203.0, ui_white_nits: 203.0 }
    }
}
impl DisplaySettings {
    pub fn sanitized(self) -> Self {
        fn finite(v: f32, fallback: f32, min: f32, max: f32) -> f32 {
            if !v.is_finite() {
                return fallback;
            }
            v.clamp(min, max)
        }
        Self {
            hdr: self.hdr,
            peak_nits: finite(self.peak_nits, 0.0, 0.0, 10000.0),
            scene_white_nits: finite(self.scene_white_nits, 203.0, 16.0, 500.0),
            ui_white_nits: finite(self.ui_white_nits, 203.0, 16.0, 500.0),
        }
    }
    pub fn peak(self, reported: Option<f32>) -> f32 {
        let requested = self.sanitized().peak_nits;
        if requested > 0.0 {
            return requested.clamp(80.0, 10000.0);
        }
        reported.filter(|v| v.is_finite() && (80.0..=10000.0).contains(v)).unwrap_or(1000.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayStatus {
    pub supported: bool,
    pub active: bool,
    pub reported_peak_nits: Option<f32>,
    pub peak_nits: f32,
    pub scene_white_nits: f32,
    pub ui_white_nits: f32,
    pub note: &'static str,
}
impl Default for DisplayStatus {
    fn default() -> Self {
        Self {
            supported: false,
            active: false,
            reported_peak_nits: None,
            peak_nits: 1000.0,
            scene_white_nits: 203.0,
            ui_white_nits: 203.0,
            note: "HDR output is unavailable on this renderer or target.",
        }
    }
}

/// Auto requires positive OS confirmation; On may opt in when metadata is unavailable.
pub fn hdr_active(mode: HdrMode, supported: bool, os_hdr: Option<bool>) -> bool {
    if !supported || os_hdr == Some(false) || mode == HdrMode::Off {
        return false;
    }
    mode == HdrMode::On || os_hdr == Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_requires_a_supported_surface_and_respects_os_sdr() {
        for mode in [HdrMode::Off, HdrMode::Auto, HdrMode::On] {
            assert!(!hdr_active(mode, false, Some(true)));
            assert!(!hdr_active(mode, true, Some(false)));
        }
        assert!(!hdr_active(HdrMode::Auto, true, None));
        assert!(hdr_active(HdrMode::On, true, None));
        assert!(hdr_active(HdrMode::Auto, true, Some(true)));
        assert!(!hdr_active(HdrMode::Off, true, Some(true)));
    }
    #[test]
    fn non_finite_calibration_and_invalid_monitor_values_use_safe_fallbacks() {
        let s = DisplaySettings {
            peak_nits: f32::INFINITY,
            scene_white_nits: f32::NAN,
            ui_white_nits: -50.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.scene_white_nits, 203.0);
        assert_eq!(s.ui_white_nits, 16.0);
        for hint in [None, Some(0.0), Some(f32::NAN), Some(f32::INFINITY)] {
            assert_eq!(s.peak(hint), 1000.0);
        }
        assert_eq!(s.peak(Some(800.0)), 800.0);
    }
}
