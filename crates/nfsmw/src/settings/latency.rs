//! Optional NVIDIA presentation controls. Stored requests survive an unavailable runtime.
pub use blackbox_gfx::ReflexMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameGeneration(pub u32);

impl FrameGeneration {
    pub const ALL: [Self; 6] = [Self(0), Self(1), Self(2), Self(3), Self(4), Self(5)];
}

impl std::fmt::Display for FrameGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 {
            return f.write_str("off");
        }
        write!(f, "{}x", self.0 + 1)
    }
}

impl std::str::FromStr for FrameGeneration {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "0" => Ok(Self(0)),
            "2x" => Ok(Self(1)),
            "3x" => Ok(Self(2)),
            "4x" => Ok(Self(3)),
            "5x" => Ok(Self(4)),
            "6x" => Ok(Self(5)),
            _ => Err("expected off, 2x, 3x, 4x, 5x or 6x".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_requests_round_trip_and_merge_without_enabling_generation_by_default() {
        let default = super::super::Settings::from(super::super::Partial::default());
        assert_eq!(default.frame_generation, FrameGeneration(0));
        for mode in FrameGeneration::ALL {
            let parsed = super::super::file::parse(&format!("frame_generation='{mode}'\nreflex='boost'"), "test");
            let s = super::super::Settings::from(parsed);
            let text = super::super::write::merge("master_volume=17", &s.graphics_layer()).unwrap();
            let restored = super::super::Settings::from(super::super::file::parse(&text, "test"));
            assert_eq!(restored.frame_generation, mode);
            assert_eq!(restored.reflex, ReflexMode::Boost);
            assert_eq!(restored.master_volume.0, 17);
        }
    }
}
