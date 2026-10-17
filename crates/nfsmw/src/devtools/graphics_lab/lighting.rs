//! Custom and original lighting controls with material dependency explanations.
use super::view::choice;
use super::*;

mod dependencies;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    ui.add_enabled_ui(lab.stats.supported, |ui| {
        ui.checkbox(&mut lab.options.freeze_sun, "Freeze sun")
            .on_hover_text("Keeps the lighting phase fixed while you drive.");
        choice(
            ui,
            "Surface debug",
            &mut lab.options.lighting_debug,
            &[
                LightingDebug::Off,
                LightingDebug::SunVisibility,
                LightingDebug::AmbientVisibility,
                LightingDebug::Cascades,
            ],
        );
        if lab.options.lighting_debug != LightingDebug::Off {
            ui.small("Debug bypasses colour effects. White means visible; black means occluded.");
        }
        ui.checkbox(&mut lab.options.sun_shadows, "Sun shadows");
        ui.add_enabled_ui(lab.options.sun_shadows, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Shadow system:");
                if ui.selectable_label(lab.options.shadow_projection.is_some(), "Original PC (partial)").clicked() {
                    dependencies::select_shadow_system(&mut lab.options, true);
                }
                if ui.selectable_label(lab.options.shadow_projection.is_none(), "Custom cascades").clicked() {
                    dependencies::select_shadow_system(&mut lab.options, false);
                }
            });
            if lab.options.shadow_projection.is_some() {
                ui.small("Single fitted perspective map; original camera and sun/cloud strength. Caster selection and vendor filtering remain incomplete. Stock size: 1024; larger maps need no new assets.");
            }
            egui::ComboBox::from_label("Map resolution")
                .selected_text(lab.options.shadow_resolution.to_string())
                .show_ui(ui, |ui| {
                    for size in [512, 1024, 2048, 4096] {
                        ui.selectable_value(&mut lab.options.shadow_resolution, size, size.to_string());
                    }
                });
            ui.add_enabled_ui(lab.options.shadow_projection.is_none(), |ui| {
                ui.checkbox(&mut lab.options.far_shadows, "Distant coverage (3× range)");
                ui.add(egui::Slider::new(&mut lab.options.shadow_distance, 40.0..=320.0).text("Near range"));
                ui.add(egui::Slider::new(&mut lab.options.shadow_filter, 0.0..=3.0).text("Filter width"));
                ui.checkbox(&mut lab.options.soft_shadows, "Contact-hardening sun (PCSS)");
                if lab.options.soft_shadows {
                    ui.add(egui::Slider::new(&mut lab.options.sun_angle, 0.0..=2.0).text("Sun radius °"));
                }
            });
        });
        advanced(ui, lab, settings);
    });
    ui.checkbox(&mut lab.options.original_shadows, "Original shadow overlays");
    if !lab.stats.supported {
        ui.colored_label(egui::Color32::YELLOW, "Prototypes require native Vulkan or DX12.");
    }
    ui.small(format!(
        "{} cascades · {} draws · {:.2} ms CPU setup · {} MiB atlas",
        lab.stats.cascades, lab.stats.shadow_draws, lab.stats.shadow_cpu_ms, lab.stats.map_mib
    ));
    if let Some(frame) = lab.stats.gpu_frame_ms {
        ui.small(format!(
            "GPU main {:.2} ms · sun {:.2} · geometry/contact {:.2} · AO {:.2}",
            frame,
            lab.stats.gpu_shadow_ms.unwrap_or(0.0),
            lab.stats.gpu_contact_ms.unwrap_or(0.0),
            lab.stats.gpu_ao_ms.unwrap_or(0.0)
        ));
    }
    if lab.stats.gpu_frame_ms.is_none() {
        ui.small("GPU timestamps unavailable or awaiting a sample");
    }
}

fn advanced(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    let dependencies = dependencies::for_selection(
        settings.car_shading,
        lab.options.original_world_materials,
        lab.options.sun_shadows && lab.options.shadow_projection.is_some(),
    );
    ui.checkbox(&mut lab.options.contact_shadows, "Contact shadow detail")
        .on_hover_text("Custom screen-space occlusion for direct-lit world surfaces; it also works without a sun map.");
    ui.add_enabled_ui(lab.options.contact_shadows, |ui| {
        ui.add(egui::Slider::new(&mut lab.options.contact_length, 0.05..=0.8).text("Contact reach"));
    });
    if !lab.options.contact_shadows {
        ui.small("Enable Contact shadow detail to adjust its reach.");
    }
    match dependencies.contact_car {
        dependencies::ContactCar::ProjectedBypass => {
            ui.small("Contact detail can affect the world here. Original PC cars bypass the contact mask while the projected map is active.");
        }
        dependencies::ContactCar::ShadowlessBypass => {
            ui.small("Contact detail can affect the world here. Original PC (shadowless) cars do not receive it.");
        }
        dependencies::ContactCar::Direct => {
            ui.small("Contact detail affects visible direct-lit receivers; flat, bright or offscreen geometry may show little change.");
        }
    }
    ui.add_enabled_ui(dependencies.custom_car, |ui| {
        ui.checkbox(&mut lab.options.clearcoat, "Car clearcoat experiment");
        ui.checkbox(&mut lab.options.car_skylight, "Car sky and ground light");
        ui.add_enabled_ui(lab.options.car_skylight, |ui| {
            ui.add(egui::Slider::new(&mut lab.options.sky_strength, 0.0..=2.0).text("Sky strength"));
            ui.add(egui::Slider::new(&mut lab.options.ground_strength, 0.0..=2.0).text("Ground bounce"));
        });
    });
    if !dependencies.custom_car {
        ui.small("Car clearcoat and sky/ground light require Custom Glossy in Materials. Original and Simple car shaders bypass these terms; stored values are retained.");
    }
    if dependencies.custom_car && !lab.options.car_skylight {
        ui.small("Enable Car sky and ground light to adjust sky strength and ground bounce.");
    }
    ui.small("World lighting uses the authored material inputs. Custom shadows and contact detail remain independent.");
}
