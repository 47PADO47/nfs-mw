//! Original tuning steps and equipment gates, applied on selection.

use nfsmw_data::car::physics::tuning::TuningCategory;

use super::Model;

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model) {
    if model.selected().is_none() {
        return;
    }
    let aero =
        model.selected().and_then(|car| car.parts.as_ref()).is_some_and(|parts| parts.aero_changed(&model.draft.parts));
    ui.weak("Unlock with performance upgrades or body kits / spoilers.");
    ui.horizontal(|ui| {
        for profile in 0..3 {
            ui.selectable_value(&mut model.draft.tuning.active, profile, format!("Profile {}", profile + 1));
        }
    });
    let Some(profile) = model.draft.tuning.profiles.get_mut(model.draft.tuning.active as usize) else {
        return;
    };
    ui.horizontal(|ui| {
        if ui.button("Reset profile").clicked() {
            *profile = [0; 7];
        }
        ui.weak("0 = neutral · 11 positions");
    });
    ui.add_space(6.0);
    for category in TuningCategory::ALL {
        let value = &mut profile[category as usize];
        let available = category.available(model.draft.performance, aero);
        ui.horizontal(|ui| {
            ui.label(category.label());
            if !available {
                ui.small("Locked").on_hover_text(category.requirement());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{value:+}"));
            });
        });
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = ui.available_width() - 12.0;
            ui.add_enabled(available, egui::Slider::new(value, -5..=5).step_by(1.0).show_value(false));
        });
        let (left, right) = category.directions();
        ui.horizontal(|ui| {
            ui.small(left);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.small(right);
            });
        });
        ui.add_space(5.0);
    }
    ui.weak("Locked values still apply. Hover Locked for requirements.");
}
