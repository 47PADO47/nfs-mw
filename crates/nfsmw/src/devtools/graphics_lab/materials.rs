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
    ui.small("Car materials and automatic lighting are configured in Lighting.");
    ui.collapsing("Advanced material paths", |ui| {
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Car material renderer").selected_text(settings.car_shading.label()).show_ui(ui, |ui| {
                for value in crate::settings::CarShading::ALL {
                    ui.selectable_value(&mut settings.car_shading, value, value.label());
                }
            });
            controls::reset(ui, &mut settings.car_shading, defaults.car_shading, "Car material renderer");
        });
        ui.small("Enhanced supports paint finishes, layered materials and environment lighting. Original PC is a reconstruction with remaining parity gaps. Simple reduces material detail.");
    });
    ui.add_enabled_ui(lab.caps.is_none_or(|c| c.local_reflections), |ui| {
        controls::choice(
            ui,
            "Car reflections",
            &mut settings.reflections,
            &ReflectionQuality::ALL,
            defaults.reflections,
        );
    });
    ui.small("Captures the car surroundings. Resolution controls detail; update rate controls motion smoothness.");
    ui.add_enabled_ui(lab.caps.is_some_and(|c| c.local_reflections), |ui| {
        controls::checkbox(ui, &mut lab.options.cached_reflections, "Cached car reflections (High: 60 Hz)", defaults.appearance.lighting.cached_reflections)
            .on_hover_text("Captures all six faces together at 60 Hz (High), 20 Hz (Medium), or 10 Hz (Low), limited by base FPS. One metre of movement or streamed scenery refreshes early. Off: High captures every base frame. No image blending.");
        controls::checkbox(ui, &mut lab.options.screen_reflections, "Selective screen-space car reflections", defaults.appearance.lighting.screen_reflections)
            .on_hover_text("Reflects visible current-frame scenery on car materials. Misses and screen edges use the local probe. Can add GPU cost; HDR and frame generation keep separate UI.");
    });
    ui.add_enabled_ui(lab.ray_reflections_available, |ui| {
        controls::checkbox(ui, &mut lab.options.ray_car_reflections, "Hardware RT car reflections", defaults.appearance.lighting.ray_car_reflections)
            .on_hover_text("Reflects loaded geometry on Enhanced car paint, glass, rims and chrome. Misses and rough finishes retain filtered cubemaps. Uses consistent approximate hit lighting, without screen-colour switching. Original PC materials retain their existing response.");
        egui::ComboBox::from_label("RT texture detail")
            .selected_text(["Low (~213 MiB)", "Balanced (~427 MiB)", "High (~1.17 GiB)", "Very High (~2.17 GiB)"][lab.options.ray_texture_quality.min(3) as usize])
            .show_ui(ui, |ui| {
                for (value, label) in ["Low", "Balanced", "High", "Very High"].into_iter().enumerate() {
                    ui.selectable_value(&mut lab.options.ray_texture_quality, value as u32, label);
                }
            }).response.on_hover_text("Shared by RT car and road reflections. Automatically gives nearby textures more detail within a fixed cache budget. Does not add rays. Changing this rebuilds the cache and can briefly pause rendering. Memory values cover RT colour textures only; source textures and RT geometry use additional VRAM.");
        ui.collapsing("Advanced RT car reflections", |ui| {
            ui.add(egui::Slider::new(&mut lab.options.ray_car_range, 40.0..=800.0).text("Car reflection ray range (m)"));
            ui.add(egui::Slider::new(&mut lab.options.ray_car_blend, 0.0..=1.0).text("Car RT geometry contribution"));
        });
    });
    super::reflections::show(ui, lab, settings);
    roads::show(ui, lab, settings);
}
