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
    let requested_renderer = settings.renderer;
    settings.renderer = match blackbox(lab, settings) {
        true => crate::settings::RendererKind::Blackbox,
        false => crate::settings::RendererKind::Bevy,
    };
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
                ui.strong(metrics.rate_line());
                ui.small(format!("{:.2} ms · {}", metrics.average_ms(), metrics.limit));
                ui.weak("F8 / Esc to close");
            });
            ui.small(format!("{} · Changes preview immediately", match blackbox(lab, settings) { true => "Black Box", false => "Bevy" }));
            profiles::header(ui, lab, settings);
            presets::show(ui, lab, settings);
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for tab in profiles::Tab::ALL { ui.selectable_value(&mut lab.tab, tab, tab.label()); }
                if !matches!(lab.tab, profiles::Tab::Environment | profiles::Tab::Diagnostics) && ui.button("Reset section").on_hover_text("Restore this section to the selected style/quality defaults. Display calibration has its own reset.").clicked() { profiles::reset(lab, settings); }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt(("graphics_section", lab.tab as u8)).max_height((ui.available_height()-8.0).max(120.0)).show(ui, |ui| {
                if !blackbox(lab, settings) && !matches!(lab.tab, profiles::Tab::Video | profiles::Tab::Environment | profiles::Tab::Diagnostics | profiles::Tab::Detail | profiles::Tab::Traffic) {
                    super::bevy::show(ui, lab, settings);
                    return;
                }
                match lab.tab {
                    profiles::Tab::Video => video::show(ui, lab, settings),
                    profiles::Tab::Look => {
                        appearance(ui, settings);
                    },
                    profiles::Tab::Lighting => lighting::show(ui, lab, settings),
                    profiles::Tab::Detail => detail::show(ui, settings),
                    profiles::Tab::Traffic => traffic::show(ui, settings, lab.traffic_status.as_deref()),
                    profiles::Tab::Materials => super::materials::show(ui, lab, settings),
                    profiles::Tab::Effects => {
                        ui.add_enabled_ui(lab.caps.is_none_or(|c| c.ambient_occlusion), |ui| {
                            let defaults = profiles::defaults(settings);
                            super::controls::choice(ui,"Ambient occlusion",&mut settings.ao,&[AoQuality::Off,AoQuality::Low,AoQuality::Medium,AoQuality::High], defaults.ao);
                            ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut settings.ao_intensity.0,0..=100).text("AO intensity %"))
                                .on_hover_text("Controls ambient occlusion strength. Direct sunlight and reflections stay unchanged; lower this to preserve baked shading.");
                            super::controls::reset(ui, &mut settings.ao_intensity, defaults.ao_intensity, "AO intensity");
                            });
                            super::controls::choice(ui,"AO method",&mut settings.ao_method,&crate::settings::AoMethod::ALL, defaults.ao_method);
                            ui.checkbox(&mut settings.ao_compare,"AO split comparison");
                            super::controls::checkbox(ui, &mut settings.ao_depth_mips,"Depth-prefiltered AO", defaults.ao_depth_mips)
                                .on_hover_text("Uses a depth-aware hierarchy for distant samples. Disable for direct-depth comparison.");
                        });
                        ui.small("GTAO, kernel SSAO and XeGTAO spatial AO are custom enhancements.");
                        effects::show(ui, settings);
                    }
                    profiles::Tab::Environment => environment::show(ui,lab,settings),
                    profiles::Tab::Diagnostics => {
                        lab.recorder.show(ui);
                        ui.separator();
                        profiles::diagnostics(ui, lab, settings);
                    },
                }
            });
        });
    lab.open = open;
    settings.renderer = requested_renderer;
}

pub(super) fn blackbox(lab: &GraphicsLab, settings: &Settings) -> bool {
    lab.caps
        .map(|caps| caps.renderer == "blackbox")
        .unwrap_or(settings.renderer == crate::settings::RendererKind::Blackbox)
}

fn appearance(ui: &mut egui::Ui, settings: &mut Settings) {
    let defaults = profiles::defaults(settings);
    super::controls::choice(
        ui,
        "SDR tone mapping",
        &mut settings.post_tonemap,
        &[PostTonemap::Off, PostTonemap::Aces],
        defaults.post_tonemap,
    );
    super::controls::choice(
        ui,
        "Bloom",
        &mut settings.post_bloom,
        &[PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High],
        defaults.post_bloom,
    );
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
