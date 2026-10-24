//! What the graphics rows' titles and data strings show: the original's label where it has the row (`OPT_DO_*`) or the
//! word (On, Off, Auto), else a message of the rewrite's catalogue (`i18n/en/options-graphics.ftl`).

use super::GraphicsSetting;
use crate::frontend::ids::{LABEL_OFF, LABEL_ON};
use crate::frontend::options::{Data, LABEL_AUTO, Title, message};
use crate::i18n::t;
use crate::settings::appearance::display::DisplayMode;
use crate::settings::reflection_extra::distance_label;
use crate::settings::reflection_res::{cube_menu_label, size_label};
use crate::settings::{
    AoQuality, GraphicsPreset, MirrorTint, ReflectionQuality, Settings, TextureFiltering, VisualTreatment,
};
use blackbox_gfx::reflection::content::ReflectionContent;
use blackbox_gfx::reflection::lod::ReflectionLod;

/// The original's titles of the rows it has (`OPT_DO_WORLD_LOD`, `OPT_DO_CAR_GEOMETRY_DETAIL`,
/// `OPT_DO_ROAD_REFLECT_DETAIL`, `OPT_DO_TEXTURE_FILTERING`).
const LABEL_WORLD_LOD: u32 = 0xA271_A8E9;
const LABEL_CAR_GEOMETRY: u32 = 0x5F56_0D76;
const LABEL_ROAD_REFLECTIONS: u32 = 0x9032_DF3F;
const LABEL_TEXTURE_FILTERING: u32 = 0xFD06_8897;

/// The On or Off label.
fn on_off(on: bool) -> Data {
    Data::Label(match on {
        true => LABEL_ON,
        false => LABEL_OFF,
    })
}

/// A quality ladder's value: the original's Off, else the catalogue's Low, Medium or High.
fn off_low_medium_high(level: usize) -> Data {
    match level {
        0 => Data::Label(LABEL_OFF),
        1 => message("options-low"),
        2 => message("options-medium"),
        _ => message("options-high"),
    }
}

/// The name of a texture filtering mode in the current language.
pub(in crate::frontend) fn filtering_name(filtering: TextureFiltering) -> String {
    let id = match filtering {
        TextureFiltering::Trilinear => "options-filtering-trilinear",
        TextureFiltering::Anisotropic8 => "options-filtering-anisotropic-8x",
        TextureFiltering::Anisotropic16 => "options-filtering-anisotropic-16x",
    };
    crate::i18n::text(id, &crate::i18n::Args::new())
}

impl GraphicsSetting {
    pub fn title(self) -> Title {
        let id = match self {
            Self::WorldDetail => return Title::Label(LABEL_WORLD_LOD),
            Self::CarGeometry => return Title::Label(LABEL_CAR_GEOMETRY),
            Self::RoadReflections => return Title::Label(LABEL_ROAD_REFLECTIONS),
            Self::TextureFiltering => return Title::Label(LABEL_TEXTURE_FILTERING),
            Self::Hdr => "options-hdr",
            Self::Style => "options-visual-style",
            Self::Quality => "options-overall-quality",
            Self::ShadowPreset => "options-shadow-quality",
            Self::ReflectionPreset => "options-reflection-quality",
            Self::Preset => "options-graphics-preset",
            Self::CarShading => "options-car-shading",
            Self::VisualTreatment => "options-visual-treatment",
            Self::Reflections => "options-car-reflections",
            Self::Shadows => "options-sun-shadows",
            Self::ShadowResolution => "options-sun-shadow-resolution",
            Self::ShadowDistance => "options-sun-shadow-distance",
            Self::ShadowCasters => "options-sun-shadow-casters",
            Self::CubeSize => "options-cube-size",
            Self::Pow2 => "options-pow2-cube",
            Self::VehicleBrightness => "options-vehicle-brightness",
            Self::ReflectionDetail => "options-reflection-detail",
            Self::ReflectionContent => "options-reflection-content",
            Self::ReflectionDistance => "options-reflection-distance",
            Self::MirrorSize => "options-mirror-size",
            Self::MirrorTint => "options-mirror-tint",
            Self::FeShadows => "options-fe-shadows",
            Self::AmbientOcclusion => "options-ambient-occlusion",
            Self::AoIntensity => "options-ao-intensity",
            Self::AoComparison => "options-ao-comparison",
        };
        Title::Key(id)
    }

    pub fn data(self, s: &Settings) -> Data {
        if let Some(data) = super::super::feature_presets::data(self, s) {
            return data;
        }
        match self {
            Self::TextureFiltering => Data::Text(filtering_name(s.texture_filtering)),
            Self::VisualTreatment => treatment(s),
            Self::AoIntensity => Data::Text(crate::i18n::fmt::percent(i64::from(s.ao_intensity.0))),
            Self::CubeSize => Data::Text(cube_menu_label(s)),
            Self::MirrorSize => Data::Text(size_label(s.reflection_mirror_size)),
            Self::VehicleBrightness => Data::Text(s.reflection_extra.brightness.to_string()),
            Self::ReflectionDistance => Data::Text(distance_label(s.reflection_extra.distance)),
            // Pixel sizes are not language.
            Self::ShadowResolution => Data::Text(format!(
                "{} × {}",
                s.appearance.lighting.shadow_resolution, s.appearance.lighting.shadow_resolution
            )),
            Self::ShadowDistance => {
                let lighting = s.appearance.lighting;
                let metres = format!("{:.0}", lighting.shadow_distance);
                Data::Text(match lighting.far_shadows {
                    true => t!("options-shadow-distance-far", metres = metres),
                    false => t!("options-shadow-distance", metres = metres),
                })
            }
            Self::ShadowCasters => match s.appearance.lighting.shadow_caster_min_radius {
                r if r <= 0.0 => message("options-shadow-casters-all"),
                r => Data::Text(t!("options-shadow-casters-radius", radius = format!("{r:.1}"))),
            },
            Self::Hdr => match s.appearance.display.mode {
                DisplayMode::Off => Data::Label(LABEL_OFF),
                DisplayMode::Auto => Data::Label(LABEL_AUTO),
                DisplayMode::On => Data::Label(LABEL_ON),
            },
            Self::ShadowPreset | Self::ReflectionPreset => unreachable!("feature preset handled above"),
            Self::Style => Data::Text(s.visual_style().menu_label()),
            Self::Quality => match s.overall_quality() {
                Some(quality) => Data::Text(quality.label()),
                None => message("options-custom"),
            },
            Self::Preset => match s.graphics_preset {
                GraphicsPreset::Custom => message("options-custom"),
                GraphicsPreset::Low => message("options-low"),
                GraphicsPreset::Medium => message("options-medium"),
                GraphicsPreset::High => message("options-high"),
                GraphicsPreset::Ultra => message("options-ultra"),
            },
            Self::Reflections => match s.reflections {
                ReflectionQuality::Off => message("options-reflections-static-sky"),
                ReflectionQuality::Low => message("options-low"),
                ReflectionQuality::Medium => message("options-medium"),
                ReflectionQuality::High => message("options-high"),
            },
            Self::Pow2 => on_off(s.reflection_pow2),
            Self::MirrorTint => mirror_tint(s.mirror_tint),
            Self::ReflectionDetail => match s.reflection_extra.lod {
                ReflectionLod::Capture => message("options-reflection-detail-capture"),
                ReflectionLod::View => message("options-reflection-detail-view"),
            },
            Self::ReflectionContent => match s.reflection_extra.content {
                ReflectionContent::Original => message("options-original"),
                ReflectionContent::Restored => message("options-reflection-content-restored"),
            },
            Self::CarShading => Data::Text(s.car_shading.menu_label()),
            Self::WorldDetail => Data::Text(s.detail.world.menu_label()),
            Self::CarGeometry => Data::Text(s.detail.car.menu_label()),
            Self::RoadReflections => Data::Text(s.road_reflections.menu_label()),
            Self::Shadows => match (s.appearance.lighting.sun_shadows, s.appearance.lighting.shadow_resolution) {
                (false, _) => off_low_medium_high(0),
                (true, 0..=1024) => off_low_medium_high(1),
                (true, 1025..=2048) => off_low_medium_high(2),
                (true, _) => off_low_medium_high(3),
            },
            Self::FeShadows => on_off(s.fe_shadows.enabled),
            Self::AmbientOcclusion => match s.ao {
                AoQuality::Off => off_low_medium_high(0),
                AoQuality::Low => off_low_medium_high(1),
                AoQuality::Medium => off_low_medium_high(2),
                AoQuality::High => off_low_medium_high(3),
            },
            Self::AoComparison => match s.ao_compare {
                true => message("options-ao-comparison-split"),
                false => Data::Label(LABEL_OFF),
            },
        }
    }
}

/// The visual treatment row: a strength set from the console or the config file reads Custom with its percent.
fn treatment(s: &Settings) -> Data {
    if let Some(strength) = s.appearance.treatment_strength {
        return Data::Text(t!("options-treatment-custom", percent = format!("{:.0}", strength * 100.0)));
    }
    match s.visual_treatment {
        VisualTreatment::Off => Data::Label(LABEL_OFF),
        VisualTreatment::Low => message("options-treatment-subtle"),
        VisualTreatment::High => message("options-original"),
        VisualTreatment::Reference => message("options-treatment-reference"),
    }
}

/// The mirror tint row: Off, the stock tint, or a custom one.
fn mirror_tint(tint: MirrorTint) -> Data {
    if tint.is_none() {
        return Data::Label(LABEL_OFF);
    }
    match tint == MirrorTint::STOCK {
        true => message("options-mirror-tint-stock"),
        false => message("options-custom"),
    }
}
