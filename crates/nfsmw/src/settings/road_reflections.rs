//! Original road-capture content with independent, asset-free resolution enhancements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RoadReflections {
    Off,
    World,
    #[default]
    Vehicles,
    Blurred,
}
names!(
    RoadReflections,
    "off, world, vehicles or blurred",
    [
        (RoadReflections::Off, "off"),
        (RoadReflections::World, "world"),
        (RoadReflections::Vehicles, "vehicles"),
        (RoadReflections::Blurred, "blurred")
    ]
);
impl RoadReflections {
    pub const ALL: [Self; 4] = [Self::Off, Self::World, Self::Vehicles, Self::Blurred];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::World => "World",
            Self::Vehicles => "World + vehicles",
            Self::Blurred => "World + vehicles + blur",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RoadReflectionResolution {
    #[default]
    Original,
    Double,
    Quadruple,
}
names!(
    RoadReflectionResolution,
    "original, 2x or 4x",
    [
        (RoadReflectionResolution::Original, "original"),
        (RoadReflectionResolution::Double, "2x"),
        (RoadReflectionResolution::Quadruple, "4x")
    ]
);
impl RoadReflectionResolution {
    pub const ALL: [Self; 3] = [Self::Original, Self::Double, Self::Quadruple];
    pub const fn size(self) -> [u32; 2] {
        match self {
            Self::Original => [320, 240],
            Self::Double => [640, 480],
            Self::Quadruple => [1280, 960],
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Original => "320 × 240 (original)",
            Self::Double => "640 × 480 (enhanced)",
            Self::Quadruple => "1280 × 960 (enhanced)",
        }
    }
}

pub(super) fn finite_scale(value: Option<f32>, fallback: f32, maximum: f32) -> f32 {
    value.filter(|v| v.is_finite()).unwrap_or(fallback).clamp(0.0, maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Settings, VisualStyle, file, write};

    #[test]
    fn rain_size_saves_merges_and_sanitizes() {
        assert_eq!(Settings::from(Partial::default()).rain_droplet_scale, 0.5);
        let lower = Partial { rain_droplet_scale: Some(0.25), ..Default::default() };
        let requested = file::parse("rain_droplet_scale = 0.75", "test");
        let s = Settings::from(requested.or(lower));
        let stored = write::merge("unknown = 'keep'", &s.appearance_layer()).unwrap();
        let restored = Settings::from(file::parse(&stored, "test"));
        assert_eq!(restored.rain_droplet_scale, 0.75);
        assert!(stored.contains("unknown = \"keep\""));
        for (value, expected) in [(f32::NAN, 0.5), (f32::INFINITY, 0.5), (-1.0, 0.0), (9.0, 2.0)] {
            let s = Settings::from(Partial { rain_droplet_scale: Some(value), ..Default::default() });
            assert_eq!(s.rain_droplet_scale, expected);
            assert_eq!(crate::settings::WorldTuning::from(&s).rain_droplet_scale, expected);
        }
    }

    #[test]
    fn road_reflections_round_trip_profiles_and_retain_unsupported_requests() {
        for quality in RoadReflections::ALL {
            for resolution in RoadReflectionResolution::ALL {
                let mut settings = Settings::from(Partial {
                    road_reflections: Some(quality),
                    road_reflection_resolution: Some(resolution),
                    ..Default::default()
                });
                settings.select_visual_style(VisualStyle::OriginalPc);
                let stored = write::merge("game_dir='keep'\nunknown='keep'", &settings.appearance_layer()).unwrap();
                let restored = Settings::from(file::parse(&stored, "test"));
                assert_eq!((restored.road_reflections, restored.road_reflection_resolution), (quality, resolution));
                assert!(stored.contains("unknown = \"keep\""));
                let mut caps = crate::settings::test_caps::native();
                caps.projected_reflections = false;
                assert_eq!(restored.effective(&caps).road_reflections, RoadReflections::Off);
                assert_eq!(restored.road_reflections, quality);
                assert_eq!(resolution.size()[0] * 3, resolution.size()[1] * 4);
            }
        }
    }
}
