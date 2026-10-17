//! Profile actions and a source/coverage inventory, independent of the renderer implementation.
use super::*;
use crate::settings::VisualStyle;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Tab {
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
    pub(super) const ALL: [Self; 8] = [
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
    ui.horizontal_wrapped(|ui| {
        ui.label("Visual style:");
        for style in VisualStyle::ALL {
            if ui.selectable_label(settings.visual_style() == style, style.label()).clicked() {
                settings.select_visual_style(style);
                lab.options = settings.appearance.lighting;
            }
        }
    });
    if settings.appearance.style == VisualStyle::OriginalPc {
        ui.small("Original PC is incomplete. See Diagnostics for remaining parity gaps.");
    }
    if settings.visual_style() == VisualStyle::Custom && settings.appearance.style != VisualStyle::Custom {
        ui.small(format!("Custom overrides based on {}.", settings.appearance.style.label()));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button("Save profile").clicked() {
            lab.save_requested = true;
        }
        if ui.button("Load saved").clicked() {
            let result = lab.profile_path.as_deref().ok_or_else(|| "No settings folder".to_owned()).and_then(|path| {
                let text = std::fs::read_to_string(path).map_err(|e| format!("Cannot load saved graphics: {e}"))?;
                saved::restore(settings, &text).map_err(|e| format!("Cannot load saved graphics: {e:#}"))?;
                lab.options = settings.appearance.lighting;
                Ok(())
            });
            lab.message = Some(match result {
                Ok(_) => "Saved graphics loaded.".into(),
                Err(e) => e,
            });
        }
        if ui.button("Reset section").clicked() {
            reset(lab, settings);
        }
    });
    if let Some(message) = &lab.message {
        ui.small(message);
    }
    if let Some(caps) = lab.caps {
        let resolved = blackbox_gfx::resolve(&settings.graphics(), &caps);
        if !resolved.downgrades.is_empty() {
            ui.small("Renderer limits affect the preview; your requested settings can still be saved and loaded.");
        }
        for downgrade in resolved.downgrades {
            if downgrade.setting == blackbox_gfx::Setting::Shadows && lab.stats.supported && lab.options.sun_shadows {
                ui.small("F8 dynamic shadows are active; the separate Video soft-shadow option is unavailable on this renderer.");
                continue;
            }
            ui.colored_label(egui::Color32::YELLOW, format!("{}: {}", downgrade.setting, downgrade.reason));
        }
    }
    saved::show(ui, lab, settings);
}
fn reset(lab: &mut GraphicsLab, settings: &mut Settings) {
    let mut defaults = *settings;
    let style = match settings.appearance.style {
        VisualStyle::Custom => VisualStyle::Enhanced,
        v => v,
    };
    defaults.select_visual_style(style);
    match lab.tab {
        Tab::Look => {
            settings.appearance.display = Default::default();
            settings.post_aa = defaults.post_aa;
            settings.post_bloom = defaults.post_bloom;
            settings.post_tonemap = defaults.post_tonemap;
            settings.visual_treatment = defaults.visual_treatment;
            settings.appearance.treatment_strength = None;
        }
        Tab::Lighting => {
            lab.options = defaults.appearance.lighting;
            lab.options.original_world_materials = settings.appearance.lighting.original_world_materials;
            lab.options.freeze_sun = settings.appearance.lighting.freeze_sun;
            lab.options.lighting_debug = settings.appearance.lighting.lighting_debug;
        }
        Tab::Materials => {
            settings.texture_filtering = crate::settings::TextureFiltering::default();
            lab.options.original_world_materials = defaults.appearance.lighting.original_world_materials;
            settings.reflections = defaults.reflections;
            settings.road_reflections = defaults.road_reflections;
            settings.road_reflection_resolution = defaults.road_reflection_resolution;
            settings.car_shading = defaults.car_shading;
        }
        Tab::Traffic => {
            settings.traffic = crate::settings::DEFAULT_TRAFFIC;
            settings.traffic_policy = Default::default();
            settings.cop_share = crate::settings::DEFAULT_COP_SHARE;
            settings.traffic_lights = crate::settings::DEFAULT_TRAFFIC_LIGHTS;
            settings.traffic_avoidance = false;
        }
        Tab::Detail => {
            settings.detail = Default::default();
        }
        Tab::Effects => {
            settings.ao = defaults.ao;
            settings.ao_intensity = defaults.ao_intensity;
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
pub(super) fn diagnostics(ui: &mut egui::Ui) {
    ui.label(crate::settings::appearance::registry::REFERENCE_POLICY);
    ui.separator();
    for feature in crate::settings::appearance::registry::FEATURES {
        ui.strong(format!("{} · {}", feature.name, feature.coverage));
        ui.label(feature.origin);
        ui.small(format!("Remaining: {}", feature.gap));
        ui.add_space(6.0);
    }
}
