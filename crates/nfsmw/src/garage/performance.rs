use super::Model;
use nfsmw_data::car::physics::{PerformanceCategory as Category, PerformancePackage};

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model) {
    let Some(car) = model.selected() else { return };
    let options = car.performance.clone();
    ui.label("Installed performance packages");
    ui.weak("Each car uses its own original stage limits.");
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Stock").clicked() {
            model.draft.performance = PerformancePackage::default();
        }
        if ui.button("Maximum").clicked() {
            model.draft.performance = options.maximum();
        }
        if ui.button("Max + Junkman").clicked() {
            let mut package = options.maximum();
            for category in Category::ALL {
                if options.junkman_available(category, package) {
                    package.junkman |= 1 << category as u8;
                }
            }
            model.draft.performance = package;
        }
    });
    ui.add_space(10.0);
    for category in [
        Category::Engine,
        Category::Transmission,
        Category::Induction,
        Category::Nitrous,
        Category::Chassis,
        Category::Tires,
        Category::Brakes,
    ] {
        let index = category as usize;
        let option = options.categories[index];
        ui.horizontal(|ui| {
            ui.label(category.label());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let eligible = options.junkman_available(category, model.draft.performance);
                let mut junkman = model.draft.performance.has_junkman(category);
                ui.add_enabled_ui(eligible, |ui| {
                    ui.checkbox(&mut junkman, "Junkman")
                        .on_hover_text("Apply this category's installed Junkman modifiers.");
                });
                let mask = 1 << category as u8;
                model.draft.performance.junkman &= !mask;
                if junkman && eligible {
                    model.draft.performance.junkman |= mask;
                }
            });
        });
        let level = &mut model.draft.performance.levels[index];
        egui::ComboBox::from_id_salt(("garage_stage", index))
            .width(ui.available_width())
            .selected_text(stage(*level, option.max_level))
            .show_ui(ui, |ui| {
                for stage_index in 0..=option.max_level {
                    ui.selectable_value(level, stage_index, stage(stage_index, option.max_level));
                }
            });
        if option.missing_endpoints {
            ui.weak("Upgrade records are missing from this install.");
        }
        if !options.junkman_available(category, model.draft.performance) {
            model.draft.performance.junkman &= !(1 << category as u8);
        }
        ui.add_space(5.0);
    }
    ui.weak("Changes apply immediately and preserve your position and speed.");
}

fn stage(level: u8, max: u8) -> String {
    if level == 0 {
        if max == 0 {
            return "Original equipment · no stage upgrades".into();
        }
        return "Original equipment".into();
    }
    if level == max {
        return format!("Stage {level} · maximum");
    }
    format!("Stage {level}")
}
