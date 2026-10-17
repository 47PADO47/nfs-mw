//! Road capture controls, separate from car cubemap quality.
use super::*;
pub(super) fn show(ui: &mut egui::Ui, lab: &GraphicsLab, settings: &mut Settings) {
    let supported = lab.caps.is_some_and(|caps| caps.projected_reflections);
    let receivers = lab.options.original_world_materials;
    ui.separator();
    ui.add_enabled_ui(supported && receivers, |ui| {
        egui::ComboBox::from_label("Road reflection detail").selected_text(settings.road_reflections.label()).show_ui(
            ui,
            |ui| {
                for value in crate::settings::RoadReflections::ALL {
                    ui.selectable_value(&mut settings.road_reflections, value, value.label());
                }
            },
        );
        ui.add_enabled_ui(settings.road_reflections != crate::settings::RoadReflections::Off, |ui| {
            egui::ComboBox::from_label("Road reflection resolution")
                .selected_text(settings.road_reflection_resolution.label())
                .show_ui(ui, |ui| {
                    for value in crate::settings::RoadReflectionResolution::ALL {
                        ui.selectable_value(&mut settings.road_reflection_resolution, value, value.label());
                    }
                });
        });
    });
    if !supported {
        ui.small("Projected road captures require the native renderer. Stored choices are retained.");
        return;
    }
    if !receivers {
        ui.small("The active material fallback cannot receive road reflections. Stored choices are retained.");
        return;
    }
    ui.small("Original authored road capture; higher image resolutions are enhancements. Rain wets the road gradually and reflections remain while it dries. Environment offers a separate comparison override.");
}
