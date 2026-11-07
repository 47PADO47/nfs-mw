//! Compact installed model selectors; searches stay inside each part's popup.

use nfsmw_data::car::customization::Category;

use super::Model;
use super::options::{Change, Edit, Inputs, Slot, Store, VehicleOptions};

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model) {
    let Some(car) = model.selected() else { return };
    let Some(catalog) = car.parts.clone() else {
        ui.weak("Model customization is unavailable for this car.");
        return;
    };
    let options = VehicleOptions::build(&Inputs {
        folder: &car.folder,
        name: &car.name,
        catalog: Some(&catalog),
        parts: &model.draft.parts,
        color: model.draft.color,
        paintable: car.paintable,
        original_color: car.original_color,
        performance: Some((&car.performance, model.draft.performance)),
    });
    ui.weak("Installed parts · changes apply immediately");
    ui.add_space(8.0);
    // The slots of the vehicle's own data: a bike's exhaust, rims and brake discs, a car's body kit, spoiler, hood, roof scoop and rims.
    let shown: Vec<&Slot> = options.slots.iter().filter(|slot| is_part(slot)).collect();
    let mut asked = None;
    for slot in &shown {
        if let Some(index) = slot_selector(ui, model, slot) {
            asked = Some((slot.id.clone(), index));
        }
    }
    if shown.is_empty() {
        ui.weak("This car has no installed parts to change.");
    }
    ui.separator();
    let changed = shown.iter().any(|slot| slot.changed());
    let restore = ui.add_enabled(changed, egui::Button::new("Restore stock parts")).clicked();
    let change = match (restore, asked) {
        (true, _) => shown.iter().map(|slot| Change::Pick { slot: slot.id.clone(), index: 0 }).collect(),
        (false, Some((slot, index))) => vec![Change::Pick { slot, index }],
        (false, None) => Vec::new(),
    };
    for change in change {
        let mut edit = Edit {
            parts: &mut model.draft.parts,
            color: &mut model.draft.color,
            performance: &mut model.draft.performance,
        };
        if let Err(error) = options.apply(Some(&catalog), &mut edit, &change) {
            model.message = Some(Err(error));
        }
    }
    clear_ineligible_rim_paint(model, &catalog);
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

/// Whether a slot is a part that is fitted to the vehicle: a model category's choice, or a component slot of its own.
fn is_part(slot: &Slot) -> bool {
    match slot.store {
        Store::Category(category) => Category::MODELS.contains(&category),
        Store::Session => true,
        Store::Level(_) => false,
    }
}

/// One slot as a list of its choices; the position the player picked, if they did.
fn slot_selector(ui: &mut egui::Ui, model: &mut Model, slot: &Slot) -> Option<usize> {
    ui.horizontal(|ui| {
        ui.label(&slot.label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match slot.choices.len() {
            0 | 1 => ui.weak("Factory only"),
            more => ui.weak(format!("{} options", more - 1)),
        });
    });
    let mut picked = None;
    let filter_at = match slot.store {
        Store::Category(category) => Some(category as usize),
        _ => None,
    };
    ui.add_enabled_ui(slot.choices.len() > 1, |ui| {
        egui::ComboBox::from_id_salt(("garage_part", &slot.id))
            .width(ui.available_width())
            .height(260.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .selected_text(&slot.installed().label)
            .show_ui(ui, |ui| {
                if let Some(at) = filter_at.filter(|_| slot.choices.len() > 10) {
                    ui.add(egui::TextEdit::singleline(&mut model.parts_filter[at]).hint_text("Search parts"));
                }
                let filter = filter_at.map(|at| model.parts_filter[at].trim().to_ascii_lowercase()).unwrap_or_default();
                for (at, choice) in slot.choices.iter().enumerate() {
                    if at != 0 && !choice.label.to_ascii_lowercase().contains(&filter) {
                        continue;
                    }
                    if ui.selectable_label(at == slot.current, &choice.label).clicked() {
                        picked = Some(at);
                        ui.close();
                    }
                }
            });
    });
    ui.add_space(10.0);
    picked
}

pub(super) fn selector(
    ui: &mut egui::Ui,
    model: &mut Model,
    catalog: &nfsmw_data::car::customization::Catalog,
    category: Category,
) {
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

fn clear_ineligible_rim_paint(model: &mut Model, catalog: &nfsmw_data::car::customization::Catalog) {
    if model.draft.parts.parts[Category::Wheels as usize].is_some_and(|h| catalog.paintable_wheels.contains(&h)) {
        return;
    }
    model.draft.parts.parts[Category::RimPaint as usize] = None;
    model.draft.parts.rim_color = None;
}
