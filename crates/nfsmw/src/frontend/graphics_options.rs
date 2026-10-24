//! The graphics-cost rows of the Video options (docs/low-end.md).

use blackbox_gfx::Capabilities;

#[cfg(test)]
use super::options::Data;
use crate::settings::reflection_extra::{cycle, step_brightness};
use crate::settings::reflection_res::{step_cube, step_size, step_tint};
use crate::settings::{
    AoQuality, CarShading, GraphicsPreset, Partial, Percent, ReflectionQuality, Settings, VisualTreatment,
};
use blackbox_gfx::reflection::content::ReflectionContent;
use blackbox_gfx::reflection::distance::ReflectionDistance;
use blackbox_gfx::reflection::lod::ReflectionLod;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsSetting {
    Hdr,
    Style,
    Quality,
    ShadowPreset,
    ReflectionPreset,
    Preset,
    CarShading,
    WorldDetail,
    CarGeometry,
    VisualTreatment,
    Reflections,
    RoadReflections,
    TextureFiltering,
    CubeSize,
    Pow2,
    VehicleBrightness,
    ReflectionDetail,
    ReflectionContent,
    ReflectionDistance,
    MirrorSize,
    MirrorTint,
    Shadows,
    ShadowResolution,
    ShadowDistance,
    ShadowCasters,
    FeShadows,
    AmbientOcclusion,
    AoIntensity,
    AoComparison,
}

impl GraphicsSetting {
    pub const ALL: [Self; 29] = [
        Self::Hdr,
        Self::Style,
        Self::Quality,
        Self::ShadowPreset,
        Self::ReflectionPreset,
        Self::Preset,
        Self::CarShading,
        Self::WorldDetail,
        Self::CarGeometry,
        Self::VisualTreatment,
        Self::TextureFiltering,
        Self::Reflections,
        Self::RoadReflections,
        Self::CubeSize,
        Self::Pow2,
        Self::VehicleBrightness,
        Self::ReflectionDetail,
        Self::ReflectionContent,
        Self::ReflectionDistance,
        Self::MirrorSize,
        Self::MirrorTint,
        Self::Shadows,
        Self::ShadowResolution,
        Self::ShadowDistance,
        Self::ShadowCasters,
        Self::FeShadows,
        Self::AmbientOcclusion,
        Self::AoIntensity,
        Self::AoComparison,
    ];

    pub fn shown(self, caps: &Capabilities) -> bool {
        match self {
            Self::ShadowPreset => Self::Shadows.shown(caps),
            Self::ReflectionPreset => caps.local_reflections,
            Self::Shadows => {
                caps.soft_shadows || (caps.renderer == "blackbox" && caps.api != blackbox_gfx::GraphicsApi::Gl)
            }
            Self::ShadowResolution | Self::ShadowDistance => {
                caps.renderer == "blackbox" && caps.api != blackbox_gfx::GraphicsApi::Gl
            }
            Self::ShadowCasters => caps.separate_shadow_casters,
            Self::VisualTreatment => caps.scene_treatment,
            Self::Reflections => caps.local_reflections,
            Self::RoadReflections => caps.projected_reflections,
            Self::FeShadows => {
                caps.soft_shadows || (caps.separate_shadow_casters && caps.api != blackbox_gfx::GraphicsApi::Gl)
            }
            Self::CubeSize
            | Self::Pow2
            | Self::VehicleBrightness
            | Self::ReflectionDetail
            | Self::ReflectionContent
            | Self::ReflectionDistance
            | Self::MirrorSize
            | Self::MirrorTint => caps.local_reflections,
            Self::AmbientOcclusion | Self::AoIntensity | Self::AoComparison => caps.ambient_occlusion,
            Self::Hdr
            | Self::Style
            | Self::Quality
            | Self::Preset
            | Self::CarShading
            | Self::TextureFiltering
            | Self::WorldDetail
            | Self::CarGeometry => true,
        }
    }

    pub fn enabled(self, s: &Settings) -> bool {
        match self {
            Self::ReflectionPreset => s.car_shading != CarShading::Simple,
            Self::ShadowResolution => s.appearance.lighting.sun_shadows,
            Self::ShadowDistance => {
                s.appearance.lighting.sun_shadows && s.appearance.lighting.shadow_projection.is_none()
            }
            Self::ShadowCasters => s.appearance.lighting.sun_shadows,
            Self::Reflections => s.car_shading != CarShading::Simple,
            Self::RoadReflections => s.appearance.lighting.original_world_materials,
            Self::AoIntensity | Self::AoComparison => s.ao != AoQuality::Off,
            Self::CubeSize | Self::Pow2 | Self::ReflectionDetail | Self::ReflectionContent => {
                s.car_shading != CarShading::Simple && s.reflections != ReflectionQuality::Off
            }
            Self::VehicleBrightness => s.car_shading != CarShading::Simple,
            Self::ReflectionDistance | Self::MirrorSize | Self::MirrorTint => true,
            Self::Style
            | Self::Hdr
            | Self::Quality
            | Self::ShadowPreset
            | Self::Preset
            | Self::CarShading
            | Self::WorldDetail
            | Self::CarGeometry
            | Self::VisualTreatment
            | Self::AmbientOcclusion
            | Self::Shadows
            | Self::TextureFiltering
            | Self::FeShadows => true,
        }
    }

    /// Moves to the next (or previous) value, wrapping, and records the change for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        if !self.enabled(s) {
            return false;
        }
        let before = *s;
        if super::feature_presets::step(self, s, changed, forward) {
            return before != *s;
        }
        match self {
            Self::ShadowResolution | Self::ShadowDistance | Self::ShadowCasters => {
                let lighting = &mut s.appearance.lighting;
                match self {
                    Self::ShadowResolution => {
                        lighting.shadow_resolution =
                            cycle(&[512, 1024, 2048, 4096, 8192], lighting.shadow_resolution, forward)
                    }
                    Self::ShadowDistance => {
                        lighting.shadow_distance = (lighting.shadow_distance
                            + match forward {
                                true => 20.0,
                                false => -20.0,
                            })
                        .clamp(40.0, 320.0)
                    }
                    Self::ShadowCasters => {
                        lighting.shadow_caster_min_radius =
                            cycle(&[0.0, 0.5, 1.0, 2.0, 3.0], lighting.shadow_caster_min_radius, forward)
                    }
                    _ => unreachable!(),
                }
                changed.appearance = Some(s.appearance);
            }
            Self::Hdr => {
                use crate::settings::appearance::display::DisplayMode;
                s.appearance.display.mode =
                    super::post_options::pick(&DisplayMode::ALL, s.appearance.display.mode, forward);
                changed.appearance = Some(s.appearance);
            }
            Self::Style => {
                let next = super::post_options::pick(&crate::settings::VisualStyle::ALL, s.visual_style(), forward);
                s.select_visual_style(next);
                *changed = s.appearance_layer().or(*changed);
            }
            Self::Quality => {
                let next = super::post_options::pick(
                    crate::settings::GraphicsQuality::offered(s.renderer),
                    s.appearance.quality,
                    forward,
                );
                s.select_overall_quality(next);
                *changed = s.appearance_layer().or(*changed);
            }
            Self::ShadowPreset | Self::ReflectionPreset => unreachable!("feature preset handled above"),
            Self::Preset => {
                let all = GraphicsPreset::ALL;
                let at = all.iter().position(|p| *p == s.graphics_preset).unwrap_or(0);
                let next = if forward { (at + 1) % all.len() } else { (at + all.len() - 1) % all.len() };
                s.apply_preset(all[next]);
                // Every key the preset sets is written, so the file says what the player sees.
                *changed = Partial { graphics_preset: Some(s.graphics_preset), ..all[next].layer() }.or(*changed);
            }
            Self::VisualTreatment => {
                s.visual_treatment = super::post_options::pick(&VisualTreatment::ALL, s.visual_treatment, forward);
                s.appearance.treatment_strength = None;
                changed.appearance = Some(s.appearance);
                changed.visual_treatment = Some(s.visual_treatment);
            }
            Self::Reflections => {
                let all = ReflectionQuality::ALL;
                let at = all.iter().position(|q| *q == s.reflections).unwrap_or(0);
                let offset = match forward {
                    true => 1,
                    false => all.len() - 1,
                };
                s.reflections = all[(at + offset) % all.len()];
                changed.reflections = Some(s.reflections);
            }
            Self::TextureFiltering => {
                s.texture_filtering =
                    super::post_options::pick(&crate::settings::TextureFiltering::ALL, s.texture_filtering, forward);
                changed.texture_filtering = Some(s.texture_filtering);
            }
            Self::RoadReflections => {
                s.road_reflections =
                    super::post_options::pick(&crate::settings::RoadReflections::ALL, s.road_reflections, forward);
                changed.road_reflections = Some(s.road_reflections);
            }
            Self::CubeSize => {
                s.reflection_cube_size = step_cube(s, forward);
                changed.reflection_cube_size = Some(s.reflection_cube_size);
            }
            Self::MirrorSize => {
                s.reflection_mirror_size = step_size(s.reflection_mirror_size, forward);
                changed.reflection_mirror_size = Some(s.reflection_mirror_size);
            }
            Self::Pow2 => {
                s.reflection_pow2 = !s.reflection_pow2;
                changed.reflection_pow2 = Some(s.reflection_pow2);
            }
            Self::MirrorTint => {
                s.mirror_tint = step_tint(s.mirror_tint);
                changed.mirror_tint = Some(s.mirror_tint);
            }
            Self::VehicleBrightness => {
                s.reflection_extra.brightness = step_brightness(s.reflection_extra.brightness, forward);
                changed.vehicle_reflection_brightness = Some(s.reflection_extra.brightness);
            }
            Self::ReflectionDetail => {
                s.reflection_extra.lod = cycle(&ReflectionLod::ALL, s.reflection_extra.lod, forward);
                changed.reflection_lod = Some(s.reflection_extra.lod);
            }
            Self::ReflectionContent => {
                s.reflection_extra.content = cycle(&ReflectionContent::ALL, s.reflection_extra.content, forward);
                changed.reflection_content = Some(s.reflection_extra.content);
            }
            Self::ReflectionDistance => {
                s.reflection_extra.distance = cycle(&ReflectionDistance::ALL, s.reflection_extra.distance, forward);
                changed.reflection_distance = Some(s.reflection_extra.distance);
            }
            Self::WorldDetail => {
                s.detail.world = super::post_options::pick(&crate::settings::WorldDetail::ALL, s.detail.world, forward);
                changed.detail = Some(s.detail);
            }
            Self::CarGeometry => {
                s.detail.car = super::post_options::pick(&crate::settings::CarGeometry::ALL, s.detail.car, forward);
                changed.detail = Some(s.detail);
            }
            Self::CarShading => {
                s.car_shading = super::post_options::pick(&CarShading::ALL, s.car_shading, forward);
                changed.car_shading = Some(s.car_shading);
            }
            Self::Shadows => {
                s.shadows = super::post_options::pick(&crate::settings::ShadowQuality::ALL, s.shadows, forward);
                s.appearance.lighting.sun_shadows = s.shadows != crate::settings::ShadowQuality::Off;
                let (size, range) = match s.shadows {
                    crate::settings::ShadowQuality::Off => (2048, 160.0),
                    crate::settings::ShadowQuality::Low => (1024, 120.0),
                    crate::settings::ShadowQuality::Medium => (2048, 200.0),
                    crate::settings::ShadowQuality::High => (4096, 320.0),
                };
                s.appearance.lighting.shadow_resolution = size;
                s.appearance.lighting.shadow_distance = range;
                changed.appearance = Some(s.appearance);
                changed.shadows = Some(s.shadows);
            }
            Self::FeShadows => {
                s.fe_shadows.enabled = !s.fe_shadows.enabled;
                changed.fe_shadows.enabled = Some(s.fe_shadows.enabled);
            }
            Self::AmbientOcclusion => {
                let at = AoQuality::ALL.iter().position(|q| *q == s.ao).unwrap_or(0);
                let offset = match forward {
                    true => 1,
                    false => AoQuality::ALL.len() - 1,
                };
                s.select_ao_budget(AoQuality::ALL[(at + offset) % AoQuality::ALL.len()]);
                changed.ao = Some(s.ao);
                changed.ao_method = Some(s.ao_method);
                changed.ao_intensity = Some(s.ao_intensity);
                changed.ao_depth_mips = Some(s.ao_depth_mips);
            }
            Self::AoIntensity => {
                s.ao_intensity = Percent(match forward {
                    true => s.ao_intensity.0.saturating_add(10).min(100),
                    false => s.ao_intensity.0.saturating_sub(10),
                });
                changed.ao_intensity = Some(s.ao_intensity);
            }
            Self::AoComparison => {
                s.ao_compare = !s.ao_compare;
                changed.ao_compare = Some(s.ao_compare);
            }
        }
        before != *s
    }
}

#[path = "graphics_options_data.rs"]
mod data;
pub(super) use data::filtering_name;

#[cfg(test)]
#[path = "graphics_options_tests.rs"]
mod tests;
