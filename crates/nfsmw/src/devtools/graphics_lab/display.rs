//! Output state and calibration are separate from artistic colour treatment.
use crate::settings::{Settings, appearance::display::DisplayMode};
pub(super) fn show(ui: &mut egui::Ui, status: blackbox_gfx::DisplayStatus, settings: &mut Settings) {
    ui.horizontal(|ui| {
        ui.heading("Display output");
        if ui.small_button("Reset display").on_hover_text("Restore Auto HDR and default display calibration").clicked()
        {
            settings.appearance.display = Default::default();
        }
    });
    let tuning = &mut settings.appearance.display;
    super::view::choice(ui, "HDR output", &mut tuning.mode, &DisplayMode::ALL);
    ui.label(match status.active {
        true => "Active: HDR / PsychoV",
        false => "Active: SDR",
    });
    ui.small(status.note);
    ui.collapsing("HDR calibration (nits)", |ui| {
        ui.add_enabled_ui(status.supported, |ui| {
            let mut automatic = tuning.peak_nits == 0.0;
            if ui.checkbox(&mut automatic, "Use display peak estimate").changed() {
                tuning.peak_nits = match automatic {
                    true => 0.0,
                    false => status.peak_nits,
                };
            }
            // Keep the zero sentinel away from Slider's unconditional range clamping,
            // which also runs while a widget is disabled.
            let mut peak = match automatic {
                true => status.peak_nits,
                false => tuning.peak_nits,
            };
            ui.add_enabled_ui(!automatic, |ui| {
                if ui.add(egui::Slider::new(&mut peak, 80.0..=10000.0).logarithmic(true).text("Peak nits")).changed()
                    && !automatic
                {
                    tuning.peak_nits = peak;
                }
            });
            ui.add(egui::Slider::new(&mut tuning.scene_white_nits, 16.0..=500.0).text("Scene white nits"));
            ui.add(egui::Slider::new(&mut tuning.ui_white_nits, 16.0..=500.0).text("UI white nits"));
        });
        ui.small(format!(
            "Effective HDR calibration: peak {:.0}, scene white {:.0}, UI white {:.0} nits.",
            status.peak_nits, status.scene_white_nits, status.ui_white_nits
        ));
        ui.small("HDR uses PsychoV after scene effects; the SDR tone mapper is retained for SDR output. PNG captures are SDR previews.");
    });
    ui.separator();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opening_hdr_controls_does_not_replace_automatic_peak_with_the_slider_minimum() {
        for supported in [true, false] {
            let ctx = egui::Context::default();
            let mut settings = Settings::from(crate::settings::Partial::default());
            let before = settings.appearance.display;
            let status =
                blackbox_gfx::DisplayStatus { supported, active: supported, peak_nits: 1350.0, ..Default::default() };
            for _ in 0..3 {
                let mut output = ctx.run_ui(Default::default(), |ui| show(ui, status, &mut settings));
                output.textures_delta.clear();
                assert_eq!(
                    settings.appearance.display, before,
                    "Opening controls must preserve Auto and stored calibration"
                );
            }
        }
    }
}
