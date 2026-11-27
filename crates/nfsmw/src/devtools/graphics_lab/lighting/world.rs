//! First enhanced world-lighting controls; detailed calibration stays in F8.
use super::*;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    let defaults = super::super::profiles::defaults(settings).appearance.lighting;
    ui.strong("World lighting");
    super::super::controls::checkbox(ui, &mut lab.options.world_skylight,
        "Directional sky and ground light", defaults.world_skylight)
        .on_hover_text("Replaces part of ambient light with broad sky colour and ground bounce. Keeps baked shading and AO. No ray tracing or frame history.");
    ui.add_enabled_ui(
        lab.options.shadow_projection.is_none() && lab.options.world_skylight && lab.options.world_skylight_strength > 0.0
            && lab.caps.is_none_or(|caps| caps.ambient_occlusion)
            && settings.ao != AoQuality::Off && settings.ao_intensity.0 > 0 && settings.ao_method.is_xe(),
        |ui| {
            super::super::controls::checkbox(ui, &mut lab.options.bent_normal_ao,
                "Directional AO (bent normals)", defaults.bent_normal_ao)
                .on_hover_text("Uses XeGTAO to steer sky and ground light toward open space. Requires XeGTAO and directional sky and ground light.");
        },
    );
    if lab.options.world_skylight && lab.options.shadow_projection.is_some() {
        ui.small(
            "Select Enhanced visual style to use this lighting. Original PC projection keeps its authored lighting.",
        );
    }
    ui.collapsing("Advanced world lighting", |ui| {
        ui.add_enabled_ui(lab.options.world_skylight, |ui| {
            super::super::controls::slider(ui, &mut lab.options.world_skylight_strength,
                0.0..=1.0, "Directional ambient balance", defaults.world_skylight_strength)
                .on_hover_text("0 keeps uniform authored ambient. 1 uses the full bounded directional response. Does not change sunlight, exposure or reflection brightness.");
            ui.small("Sky colour follows sun and cloud coverage. Ground bounce is a broad estimate; local buildings and tunnels retain their baked shading.");
        });
    });
}
