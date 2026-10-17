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
            ui.small("Changes preview immediately. Save profile keeps graphics and traffic for the next launch.");
            profiles::header(ui, lab, settings);
            presets::show(ui, lab, settings);
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for tab in profiles::Tab::ALL { ui.selectable_value(&mut lab.tab, tab, tab.label()); }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt(("graphics_section", lab.tab as u8)).max_height((ui.available_height()-8.0).max(120.0)).show(ui, |ui| {
                match lab.tab {
                    profiles::Tab::Look => {
                        display::show(ui, lab.display, settings);
                        appearance(ui, settings);
                    },
                    profiles::Tab::Lighting => lighting::show(ui, lab, settings),
                    profiles::Tab::Detail => detail::show(ui, settings),
                    profiles::Tab::Traffic => traffic::show(ui, settings, lab.traffic_status.as_deref()),
                    profiles::Tab::Materials => {
                        egui::ComboBox::from_label("Texture filtering").selected_text(settings.texture_filtering.label()).show_ui(ui, |ui| {
                            for filtering in crate::settings::TextureFiltering::ALL {
                                ui.selectable_value(&mut settings.texture_filtering, filtering, filtering.label());
                            }
                        });
                        ui.small("Trilinear or 8x/16x anisotropic filtering for scene surfaces, including normal-mapped roads. Authored mip levels remain in use.");
                        if lab.caps.is_some_and(|c| c.restart_required.contains(blackbox_gfx::Setting::TextureFiltering)) {
                            ui.small("This renderer applies texture filtering after restart. Save profile to keep the selection.");
                        }
                        ui.label("Authored world materials");
                        ui.small("Common, normal, reflective-road and glass families form the shared world baseline. Unsupported effects use a simple fallback.");
                        egui::ComboBox::from_label("Car shading").selected_text(settings.car_shading.label()).show_ui(ui, |ui| {
                            for shading in crate::settings::CarShading::ALL {
                                ui.selectable_value(&mut settings.car_shading, shading, shading.label());
                            }
                        });
                        if settings.car_shading.is_original() && lab.caps.is_some_and(|c| c.renderer != "blackbox") {
                            ui.small("This renderer uses a custom shader fallback. Original car material arithmetic requires the native renderer.");
                        }
                        ui.small("Original PC uses the regular PC material family; shadowless uses the low-shadow family. Select the separate shadow system in Lighting. Custom Glossy retains the enhanced rig.");
                        ui.add_enabled_ui(lab.caps.is_none_or(|c| c.local_reflections), |ui| {
                            choice(ui,"Car reflections",&mut settings.reflections,&[ReflectionQuality::Off,ReflectionQuality::Low,ReflectionQuality::Medium,ReflectionQuality::High]);
                        });
                        ui.small("Car environment captures; resolution/update budgets are custom.");
                        roads::show(ui, lab, settings);
                    }
                    profiles::Tab::Effects => {
                        ui.add_enabled_ui(lab.caps.is_none_or(|c| c.ambient_occlusion), |ui| {
                            choice(ui,"Ambient occlusion",&mut settings.ao,&[AoQuality::Off,AoQuality::Low,AoQuality::Medium,AoQuality::High]);
                            ui.add(egui::Slider::new(&mut settings.ao_intensity.0,0..=100).text("AO intensity %"));
                            ui.checkbox(&mut settings.ao_compare,"AO split comparison");
                        });
                        ui.small("GTAO is a custom enhancement. Particle preferences remain in the Video menu and are not reset by style/quality.");
                    }
                    profiles::Tab::Environment => environment::show(ui,lab,settings),
                    profiles::Tab::Diagnostics => profiles::diagnostics(ui),
                }
            });
        });
    lab.open = open;
}

fn appearance(ui: &mut egui::Ui, settings: &mut Settings) {
    choice(ui, "SDR tone mapping", &mut settings.post_tonemap, &[PostTonemap::Off, PostTonemap::Aces]);
    choice(
        ui,
        "Bloom",
        &mut settings.post_bloom,
        &[PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High],
    );
    choice(ui, "AA", &mut settings.post_aa, &[PostAa::Off, PostAa::Fxaa, PostAa::Smaa, PostAa::Taa]);
    ui.horizontal_wrapped(|ui| {
        ui.label("Visual Treatment:");
        for (label, value) in [
            ("Off", VisualTreatment::Off),
            ("Subtle", VisualTreatment::Low),
            ("Original", VisualTreatment::High),
            ("Reference (mod)", VisualTreatment::Reference),
        ] {
            if ui
                .selectable_label(
                    settings.appearance.treatment_strength.is_none() && settings.visual_treatment == value,
                    label,
                )
                .clicked()
            {
                settings.visual_treatment = value;
                settings.appearance.treatment_strength = None;
            }
        }
    });
    let mut strength = settings.appearance.treatment_strength.unwrap_or_else(|| settings.visual_treatment.strength());
    if ui.add(egui::Slider::new(&mut strength, 0.0..=1.0).text("Treatment strength")).changed() {
        settings.appearance.treatment_strength = Some(strength);
    }
    if settings.visual_treatment == VisualTreatment::Reference {
        ui.small(
            "Supplied mod override: RGB 0.68 / 0.62 / 0.03; colour bloom 0.75; Saturation 0.80; black bloom 0.40.",
        );
        ui.small(
            "Uses installed curves and gain map. Saturation 0.80 retains 80% source colour before bloom and gain.",
        );
    }
    ui.small("Original midday profile only. Tunnel/pursuit look selection and original Over Bright remain incomplete; current bloom and ACES are custom.");
}
pub(super) fn choice<T: Copy + PartialEq + std::fmt::Display>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[T],
) {
    egui::ComboBox::from_label(label).selected_text(value.to_string()).show_ui(ui, |ui| {
        for choice in choices {
            ui.selectable_value(value, *choice, choice.to_string());
        }
    });
}
