//! The graphics-cost rows of the Video options (docs/low-end.md).

use blackbox_gfx::Capabilities;

use super::options::{Data, Title};
use crate::settings::reflection_extra::{content_label, cycle, distance_label, lod_label, step_brightness};
use crate::settings::reflection_res::{cube_menu_label, size_label, step_cube, step_size, step_tint, tint_label};
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

    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Hdr => "HDR Output (Auto / On / Off)",
            Self::Style => "Visual Style",
            Self::Quality => "Overall Quality",
            Self::ShadowPreset => "Shadow Quality",
            Self::ReflectionPreset => "Reflection Quality",
            Self::Preset => "Graphics Preset",
            Self::CarShading => "Car Shading",
            Self::WorldDetail => "World Level of Detail",
            Self::CarGeometry => "Car Geometry Detail",
            Self::VisualTreatment => "Visual Treatment (Experimental)",
            Self::Reflections => "Car Reflections",
            Self::RoadReflections => "Road Reflection Detail",
            Self::TextureFiltering => "Texture Filtering",
            Self::Shadows => "Enhanced Sun Shadows",
            Self::ShadowResolution => "Sun Shadow Base Resolution",
            Self::ShadowDistance => "Sun Shadow Distance",
            Self::ShadowCasters => "Sun Shadow Caster Detail",
            Self::CubeSize => "Car Cubemap Resolution (Per Face)",
            Self::Pow2 => "Power-of-Two Cube Size",
            Self::VehicleBrightness => "Vehicle Reflection Brightness",
            Self::ReflectionDetail => "Reflection Detail",
            Self::ReflectionContent => "Reflection Content",
            Self::ReflectionDistance => "Reflection Distance",
            Self::MirrorSize => "Mirror Reflection Size",
            Self::MirrorTint => "Mirror Tint",
            Self::FeShadows => "Front-End Shadows",
            Self::AmbientOcclusion => "Ambient Occlusion",
            Self::AoIntensity => "AO Intensity",
            Self::AoComparison => "AO Comparison",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        if let Some(data) = super::feature_presets::data(self, s) {
            return data;
        }
        if self == Self::TextureFiltering {
            return Data::Text(s.texture_filtering.label().into());
        }
        if self == Self::VisualTreatment
            && let Some(strength) = s.appearance.treatment_strength
        {
            return Data::Text(format!("Custom ({:.0}%)", strength * 100.0));
        }
        if self == Self::AoIntensity {
            return Data::Text(format!("{}%", s.ao_intensity.0));
        }
        if self == Self::CubeSize {
            return Data::Text(cube_menu_label(s));
        }
        if self == Self::MirrorSize {
            return Data::Text(size_label(s.reflection_mirror_size));
        }
        if self == Self::VehicleBrightness {
            return Data::Text(s.reflection_extra.brightness.to_string());
        }
        if self == Self::ReflectionDistance {
            return Data::Text(distance_label(s.reflection_extra.distance));
        }
        if self == Self::ShadowResolution {
            return Data::Text(format!(
                "{} × {}",
                s.appearance.lighting.shadow_resolution, s.appearance.lighting.shadow_resolution
            ));
        }
        if self == Self::ShadowDistance {
            let lighting = s.appearance.lighting;
            return Data::Text(format!(
                "{:.0} m{}",
                lighting.shadow_distance,
                match lighting.far_shadows {
                    true => " (+ distant coverage)",
                    false => "",
                }
            ));
        }
        if self == Self::ShadowCasters {
            let radius = s.appearance.lighting.shadow_caster_min_radius;
            return Data::Text(match radius {
                r if r <= 0.0 => "All scenery".into(),
                r => format!("Scenery radius ≥ {r:.1} m"),
            });
        }
        Data::Text(
            match self {
                Self::Hdr => match s.appearance.display.mode {
                    crate::settings::appearance::display::DisplayMode::Off => "Off",
                    crate::settings::appearance::display::DisplayMode::Auto => "Auto",
                    crate::settings::appearance::display::DisplayMode::On => "On",
                },
                Self::ShadowPreset | Self::ReflectionPreset => unreachable!("feature preset handled above"),
                Self::Style => s.visual_style().label(),
                Self::Quality => s.overall_quality().map_or("Custom", |q| q.label()),
                Self::Preset => match s.graphics_preset {
                    GraphicsPreset::Custom => "Custom",
                    GraphicsPreset::Low => "Low",
                    GraphicsPreset::Medium => "Medium",
                    GraphicsPreset::High => "High",
                    GraphicsPreset::Ultra => "Ultra",
                },
                Self::VisualTreatment => match s.visual_treatment {
                    VisualTreatment::Off => "Off",
                    VisualTreatment::Low => "Subtle",
                    VisualTreatment::High => "Original",
                    VisualTreatment::Reference => "Reference (mod)",
                },
                Self::Reflections => match s.reflections {
                    ReflectionQuality::Off => "Static Sky",
                    ReflectionQuality::Low => "Low",
                    ReflectionQuality::Medium => "Medium",
                    ReflectionQuality::High => "High",
                },
                Self::CubeSize | Self::MirrorSize | Self::VehicleBrightness => unreachable!("labelled above"),
                Self::Pow2 => match s.reflection_pow2 {
                    true => "On",
                    false => "Off",
                },
                Self::MirrorTint => tint_label(s.mirror_tint),
                Self::ReflectionDetail => lod_label(s.reflection_extra.lod),
                Self::ReflectionContent => content_label(s.reflection_extra.content),
                Self::ReflectionDistance => unreachable!("labelled above"),
                Self::ShadowResolution | Self::ShadowDistance | Self::ShadowCasters => unreachable!("labelled above"),
                Self::CarShading => s.car_shading.label(),
                Self::WorldDetail => s.detail.world.label(),
                Self::CarGeometry => s.detail.car.label(),
                Self::RoadReflections => s.road_reflections.label(),
                Self::Shadows => match (s.appearance.lighting.sun_shadows, s.appearance.lighting.shadow_resolution) {
                    (false, _) => "Off",
                    (true, 0..=1024) => "Low",
                    (true, 1025..=2048) => "Medium",
                    (true, _) => "High",
                },
                Self::FeShadows => match s.fe_shadows.enabled {
                    true => "On",
                    false => "Off",
                },
                Self::AmbientOcclusion => match s.ao {
                    AoQuality::Off => "Off",
                    AoQuality::Low => "Low",
                    AoQuality::Medium => "Medium",
                    AoQuality::High => "High",
                },
                Self::AoComparison => match s.ao_compare {
                    true => "Split Screen",
                    false => "Off",
                },
                Self::AoIntensity | Self::TextureFiltering => unreachable!("special value handled above"),
            }
            .to_owned(),
        )
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

#[cfg(test)]
#[path = "graphics_options_tests.rs"]
mod tests;
