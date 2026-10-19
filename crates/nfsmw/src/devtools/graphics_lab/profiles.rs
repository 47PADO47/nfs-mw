//! Profile actions and a source/coverage inventory, independent of the renderer implementation.
use super::*;
use crate::settings::VisualStyle;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Tab {
    Video,
    #[default]
    Look,
    Lighting,
    Materials,
    Detail,
    Traffic,
    Effects,
    Environment,
    Diagnostics,
}
impl Tab {
    pub(super) const ALL: [Self; 9] = [
        Self::Video,
        Self::Look,
        Self::Lighting,
        Self::Materials,
        Self::Detail,
        Self::Traffic,
        Self::Effects,
        Self::Environment,
        Self::Diagnostics,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Look => "Look",
            Self::Lighting => "Lighting",
            Self::Materials => "Materials",
            Self::Detail => "Detail",
            Self::Traffic => "Traffic",
            Self::Effects => "Effects",
            Self::Environment => "Environment",
            Self::Diagnostics => "Diagnostics",
        }
    }
}
pub(super) fn header(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    saved::show(ui, lab, settings);
    if let Some(message) = &lab.message {
        ui.small(message);
    }
    if !super::view::blackbox(lab, settings) {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Visual style:");
        for style in VisualStyle::ALL {
            if ui.selectable_label(settings.visual_style() == style, style.label()).clicked() {
                settings.select_visual_style(style);
                lab.options = settings.appearance.lighting;
            }
        }
    });
}

pub(super) fn reset(lab: &mut GraphicsLab, settings: &mut Settings) {
    if !super::view::blackbox(lab, settings) && !matches!(lab.tab, Tab::Video | Tab::Detail | Tab::Traffic) {
        super::bevy::reset(lab.tab, settings);
        return;
    }
    let defaults = defaults(settings);
    match lab.tab {
        Tab::Video => super::video::reset(settings),
        Tab::Look => {
            settings.post_bloom = defaults.post_bloom;
            settings.post_tonemap = defaults.post_tonemap;
            settings.visual_treatment = defaults.visual_treatment;
            settings.appearance.treatment_strength = None;
        }
        Tab::Lighting => {
            let reflections = (lab.options.cached_reflections, lab.options.screen_reflections);
            lab.options = defaults.appearance.lighting;
            lab.options.cached_reflections = reflections.0;
            lab.options.screen_reflections = reflections.1;
            lab.options.original_world_materials = settings.appearance.lighting.original_world_materials;
            lab.options.freeze_sun = settings.appearance.lighting.freeze_sun;
            lab.options.lighting_debug = settings.appearance.lighting.lighting_debug;
        }
        Tab::Materials => {
            lab.options.cached_reflections = defaults.appearance.lighting.cached_reflections;
            lab.options.screen_reflections = defaults.appearance.lighting.screen_reflections;
            settings.texture_filtering = defaults.texture_filtering;
            lab.options.original_world_materials = defaults.appearance.lighting.original_world_materials;
            settings.reflections = defaults.reflections;
            settings.road_reflections = defaults.road_reflections;
            settings.road_reflection_resolution = defaults.road_reflection_resolution;
            settings.car_shading = defaults.car_shading;
            settings.reflection_cube_size = defaults.reflection_cube_size;
            settings.reflection_mirror_size = defaults.reflection_mirror_size;
            settings.reflection_pow2 = defaults.reflection_pow2;
            settings.mirror_tint = defaults.mirror_tint;
            settings.reflection_extra = defaults.reflection_extra;
        }
        Tab::Traffic => {
            settings.traffic = crate::settings::DEFAULT_TRAFFIC;
            settings.traffic_policy = Default::default();
            settings.cop_share = crate::settings::DEFAULT_COP_SHARE;
            settings.traffic_lights = crate::settings::DEFAULT_TRAFFIC_LIGHTS;
            settings.traffic_avoidance = false;
        }
        Tab::Detail => {
            settings.detail = defaults.detail;
        }
        Tab::Effects => {
            settings.ao = defaults.ao;
            settings.ao_intensity = defaults.ao_intensity;
            settings.ao_depth_mips = defaults.ao_depth_mips;
            settings.ao_method = defaults.ao_method;
            settings.ao_compare = false;
        }
        Tab::Diagnostics => {
            lab.options.lighting_debug = LightingDebug::Off;
            settings.ao_compare = false;
        }
        Tab::Environment => {
            lab.message =
                Some("Time/weather overrides are independent; choose sun mode, clouds and rain in Environment.".into());
        }
    }
}
pub(super) fn defaults(settings: &Settings) -> Settings {
    let mut defaults = Settings::from(crate::settings::Partial::default());
    defaults.renderer = settings.renderer;
    defaults.appearance.quality = settings.appearance.quality;
    let style = match settings.appearance.style {
        VisualStyle::Custom => VisualStyle::Enhanced,
        v => v,
    };
    defaults.select_visual_style(style);
    defaults
}

pub(super) fn diagnostics(ui: &mut egui::Ui, lab: &GraphicsLab, settings: &Settings) {
    if let Some(caps) = lab.caps {
        for downgrade in blackbox_gfx::resolve(&settings.graphics(), &caps).downgrades {
            ui.colored_label(egui::Color32::YELLOW, format!("{}: {}", downgrade.setting, downgrade.reason));
        }
    }
    if !super::view::blackbox(lab, settings) {
        ui.label("Bevy controls follow the active renderer's capabilities. Black Box experiments remain stored.");
        return;
    }
    ui.label(crate::settings::appearance::registry::REFERENCE_POLICY);
    ui.separator();
    for feature in crate::settings::appearance::registry::FEATURES {
        ui.strong(format!("{} · {}", feature.name, feature.coverage));
        ui.label(feature.origin);
        ui.small(format!("Remaining: {}", feature.gap));
        ui.add_space(6.0);
    }
}
