//! Compact installed model selectors; searches stay inside each part's popup.

use nfsmw_data::car::customization::{Category, Selection};

use super::Model;

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model) {
    let Some(car) = model.selected() else { return };
    let Some(catalog) = car.parts.clone() else {
        ui.weak("Model customization is unavailable for this car.");
        return;
    };
    ui.weak("Installed parts · changes apply immediately");
    ui.add_space(8.0);
    for category in Category::ALL {
        let index = category as usize;
        let choices = &catalog.choices[index];
        let hash = &mut model.draft.parts.parts[index];
        let selected = hash.and_then(|h| choices.iter().find(|p| p.hash == h)).map_or("Stock", |p| p.label.as_str());
        ui.horizontal(|ui| {
            ui.label(category.label());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if choices.is_empty() {
                    ui.weak("Factory only");
                }
                if !choices.is_empty() {
                    ui.weak(format!("{} options", choices.len()));
                }
            });
        });
        ui.add_enabled_ui(!choices.is_empty(), |ui| {
            egui::ComboBox::from_id_salt(("garage_part", index))
                .width(ui.available_width())
                .height(260.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if choices.len() > 10 {
                        ui.add(egui::TextEdit::singleline(&mut model.parts_filter[index]).hint_text("Search parts"));
                    }
                    if ui.selectable_value(hash, None, "Stock").clicked() {
                        ui.close();
                    }
                    let filter = model.parts_filter[index].trim().to_ascii_lowercase();
                    for part in choices.iter().filter(|p| p.label.to_ascii_lowercase().contains(&filter)) {
                        if ui.selectable_value(hash, Some(part.hash), &part.label).clicked() {
                            ui.close();
                        }
                    }
                });
        });
        ui.add_space(10.0);
    }
    ui.separator();
    if ui.add_enabled(!model.draft.parts.is_stock(), egui::Button::new("Restore stock parts")).clicked() {
        model.draft.parts = Selection::default();
    }
    ui.add_space(8.0);
    match catalog.aero_changed(&model.draft.parts) {
        true => {
            ui.weak("Aerodynamics tuning unlocked in Tuning.");
        }
        false => {
            ui.weak("Fit a body kit or spoiler to unlock aerodynamics tuning.");
        }
    }
}
