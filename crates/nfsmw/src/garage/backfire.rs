//! Backfire appearance is independent of the continuous NOS style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackfireEvent {
    Shift,
    Pop,
}

pub(super) fn show(ui: &mut egui::Ui, model: &mut super::Model, settings: &mut crate::settings::Settings) {
    let before = settings.backfire_style;
    ui.collapsing("Backfire", |ui| {
        ui.add_enabled_ui(settings.enhanced_exhaust && settings.exhaust_flames, |ui| {
            ui.horizontal(|ui| {
                for (label, event) in [("Preview shift", BackfireEvent::Shift), ("Preview pop", BackfireEvent::Pop)] {
                    if ui.button(label).clicked() {
                        model.exhaust_preview = false;
                        model.backfire_preview = Some(event);
                    }
                }
            });
        });
        let style = &mut settings.backfire_style;
        ui.horizontal(|ui| {
            ui.label("Root tint");
            ui.color_edit_button_srgb(&mut style.root);
        });
        ui.horizontal(|ui| {
            ui.label("Tail tint");
            ui.color_edit_button_srgb(&mut style.tail);
        });
        for (label, field) in [
            ("Size", &mut style.size),
            ("Brightness", &mut style.brightness),
            ("Duration", &mut style.duration),
            ("Variation", &mut style.variation),
        ] {
            ui.add(egui::Slider::new(field, 0..=200).text(label).suffix("%"));
        }
        if ui.button("Reset backfire").clicked() {
            *style = Default::default();
        }
        ui.weak("Shared across cars. Preview flashes do not change the engine or consume NOS.");
    });
    model.exhaust_style_dirty |= before != settings.backfire_style;
}
