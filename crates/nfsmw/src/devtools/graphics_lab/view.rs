//! Resizable graphics lab with separate experiment and image controls.
use super::*;

pub(super) fn show(
    ctx: &egui::Context,
    lab: &mut GraphicsLab,
    settings: &mut Settings,
    metrics: &super::super::metrics::Metrics,
) {
    lab.remember_startup(settings);
    if !lab.open {
        return;
    }
    let mut open = lab.open;
    egui::Window::new("Graphics lab · F8")
        .default_pos(egui::pos2(24.0, 48.0))
        .default_size(egui::vec2(900.0, 700.0))
        .min_size(egui::vec2(360.0, 360.0))
        .resizable(true)
        .max_height((ctx.content_rect().height() - 70.0).max(300.0))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!(
                    "{:.0} FPS · {:.2} ms · worst {:.2} ms",
                    metrics.fps(),
                    metrics.average_ms(),
                    metrics.worst_ms()
                ));
                ui.weak("F8 / Esc to close");
            });
            ui.small("Changes apply immediately for this session. Close the panel to drive.");
            presets::show(ui, lab, settings);
            ui.separator();
            let height = (ui.available_height() - 12.0).max(200.0);
            if ui.available_width() < 650.0 {
                egui::ScrollArea::vertical().id_salt("lab_narrow").max_height(height).show(ui, |ui| {
                    ui.heading("Lighting experiments");
                    experiments(ui, lab);
                    ui.separator();
                    ui.heading("Image settings");
                    appearance(ui, settings);
                    environment::show(ui, lab, settings);
                });
                return;
            }
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical().id_salt("lab_experiments").max_height(height).show(
                    &mut columns[0],
                    |ui| {
                        ui.heading("Lighting experiments");
                        experiments(ui, lab);
                    },
                );
                egui::ScrollArea::vertical().id_salt("lab_image").max_height(height).show(&mut columns[1], |ui| {
                    ui.heading("Image settings");
                    appearance(ui, settings);
                    environment::show(ui, lab, settings);
                });
            });
        });
    lab.open = open;
}

fn experiments(ui: &mut egui::Ui, lab: &mut GraphicsLab) {
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
            ui.checkbox(&mut lab.options.far_shadows, "Distant coverage (3× range)");
            egui::ComboBox::from_label("Map resolution")
                .selected_text(lab.options.shadow_resolution.to_string())
                .show_ui(ui, |ui| {
                    for size in [512, 1024, 2048, 4096] {
                        ui.selectable_value(&mut lab.options.shadow_resolution, size, size.to_string());
                    }
                });
            ui.add(egui::Slider::new(&mut lab.options.shadow_distance, 40.0..=320.0).text("Near range"));
            ui.add(egui::Slider::new(&mut lab.options.shadow_filter, 0.0..=3.0).text("Filter width"));
            ui.checkbox(&mut lab.options.soft_shadows, "Contact-hardening sun (PCSS)");
            if lab.options.soft_shadows {
                ui.add(egui::Slider::new(&mut lab.options.sun_angle, 0.0..=2.0).text("Sun radius °"));
            }
        });
        ui.checkbox(&mut lab.options.contact_shadows, "Contact shadow detail");
        if lab.options.contact_shadows {
            ui.add(egui::Slider::new(&mut lab.options.contact_length, 0.05..=0.8).text("Contact reach"));
        }
        ui.checkbox(&mut lab.options.clearcoat, "Car clearcoat experiment");
        ui.checkbox(&mut lab.options.car_skylight, "Car sky and ground light");
        if lab.options.car_skylight {
            ui.add(egui::Slider::new(&mut lab.options.sky_strength, 0.0..=2.0).text("Sky strength"));
            ui.add(egui::Slider::new(&mut lab.options.ground_strength, 0.0..=2.0).text("Ground bounce"));
        }
        ui.checkbox(&mut lab.options.world_ambient, "World ambient lighting");
        if lab.options.world_ambient {
            ui.add(egui::Slider::new(&mut lab.options.world_ambient_share, 0.35..=0.9).text("Shade fill"));
            ui.add(egui::Slider::new(&mut lab.options.world_ambient_tint, 0.0..=1.0).text("Ambient colour"));
        }
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

fn appearance(ui: &mut egui::Ui, settings: &mut Settings) {
    choice(
        ui,
        "Car reflections",
        &mut settings.reflections,
        &[ReflectionQuality::Off, ReflectionQuality::Low, ReflectionQuality::Medium, ReflectionQuality::High],
    );
    choice(
        ui,
        "Ambient occlusion",
        &mut settings.ao,
        &[AoQuality::Off, AoQuality::Low, AoQuality::Medium, AoQuality::High],
    );
    choice(ui, "Tone mapping", &mut settings.post_tonemap, &[PostTonemap::Off, PostTonemap::Aces]);
    ui.small("AO affects ambient light; sunlight and reflections stay intact.");
    ui.checkbox(&mut settings.ao_compare, "AO split comparison");
    ui.add(egui::Slider::new(&mut settings.ao_intensity.0, 0..=100).text("AO intensity %"));
    choice(
        ui,
        "Bloom",
        &mut settings.post_bloom,
        &[PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High],
    );
    choice(ui, "AA", &mut settings.post_aa, &[PostAa::Off, PostAa::Fxaa, PostAa::Smaa]);
    choice(
        ui,
        "Visual Treatment",
        &mut settings.visual_treatment,
        &[VisualTreatment::Off, VisualTreatment::Low, VisualTreatment::High],
    );
}
fn choice<T: Copy + PartialEq + std::fmt::Display>(ui: &mut egui::Ui, label: &str, value: &mut T, choices: &[T]) {
    egui::ComboBox::from_label(label).selected_text(value.to_string()).show_ui(ui, |ui| {
        for choice in choices {
            ui.selectable_value(value, *choice, choice.to_string());
        }
    });
}
