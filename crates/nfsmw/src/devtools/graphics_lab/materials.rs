//! Material and capture controls for the native renderer.
use super::*;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    let defaults = profiles::defaults(settings);
    ui.horizontal(|ui| {
        egui::ComboBox::from_label("Texture filtering").selected_text(settings.texture_filtering.label()).show_ui(
            ui,
            |ui| {
                for value in crate::settings::TextureFiltering::ALL {
                    ui.selectable_value(&mut settings.texture_filtering, value, value.label());
                }
            },
        );
        controls::reset(ui, &mut settings.texture_filtering, defaults.texture_filtering, "Texture filtering");
    });
    ui.small("Trilinear or 8×/16× anisotropic filtering, including normal-mapped roads. Quality presets keep 16×.");
    if lab.caps.is_some_and(|c| c.restart_required.contains(blackbox_gfx::Setting::TextureFiltering)) {
        ui.small("Texture filtering applies after restart. Save for startup to keep the selection.");
    }
    ui.horizontal(|ui| {
        egui::ComboBox::from_label("Car shading").selected_text(settings.car_shading.label()).show_ui(ui, |ui| {
            for value in crate::settings::CarShading::ALL {
                ui.selectable_value(&mut settings.car_shading, value, value.label());
            }
        });
        controls::reset(ui, &mut settings.car_shading, defaults.car_shading, "Car shading");
    });
    ui.small("Original PC uses the PC material family; shadowless uses the low-shadow family. Custom Glossy supports clearcoat and sky/ground light.");
    ui.add_enabled_ui(lab.caps.is_none_or(|c| c.local_reflections), |ui| {
        controls::choice(
            ui,
            "Car reflections",
            &mut settings.reflections,
            &ReflectionQuality::ALL,
            defaults.reflections,
        );
    });
    ui.small("Car environment captures; resolution and update budgets are custom.");
    ui.add_enabled_ui(lab.caps.is_some_and(|c| c.local_reflections), |ui| {
        controls::checkbox(ui, &mut lab.options.cached_reflections, "Cached car reflections (High: 60 Hz)", defaults.appearance.lighting.cached_reflections)
            .on_hover_text("Captures all six faces together at 60 Hz (High), 20 Hz (Medium), or 10 Hz (Low), limited by base FPS. One metre of movement or streamed scenery refreshes early. Off: High captures every base frame. No image blending.");
        controls::checkbox(ui, &mut lab.options.screen_reflections, "Selective screen-space car reflections", defaults.appearance.lighting.screen_reflections)
            .on_hover_text("Reflects visible current-frame scenery on car materials. Misses and screen edges use the local probe. Can add GPU cost; HDR and frame generation keep separate UI.");
    });
    super::reflections::show(ui, lab, settings);
    roads::show(ui, lab, settings);
}
