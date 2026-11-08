//! What the F6 garage shows beside the car browser: the way to the vehicle panel (F11), where everything about the current vehicle's
//! setup is, and the flame styles, which are shared across cars and not part of a vehicle's setup.

use super::Model;
use crate::settings::Settings;

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model, settings: &mut Settings) {
    let selected = model.selected();
    let title = selected.map_or(model.draft.car.as_str(), |car| car.name.as_str());
    ui.heading(title);
    if let Some(car) = selected {
        let subtitle = match car.category {
            super::Category::Racing => car.manufacturer.clone(),
            super::Category::Police => "Police vehicle".to_owned(),
        };
        ui.weak(subtitle);
    }
    ui.add_space(12.0);
    if ui.button("Vehicle options: open the vehicle panel (F11)").clicked() {
        model.vehicle_panel_requested = true;
    }
    ui.weak("Parts, appearance, performance, tuning, saved setups and the rider's gear are on the vehicle panel.");
    ui.add_space(12.0);
    ui.separator();
    model.draft.exhaust.set_style(settings.exhaust_style);
    super::exhaust::show(ui, &mut model.draft.exhaust, settings.enhanced_exhaust, &mut model.exhaust_preview);
    let style = model.draft.exhaust.style();
    if style != settings.exhaust_style {
        settings.exhaust_style = style;
        model.exhaust_style_dirty = true;
    }
    super::backfire::show(ui, model, settings);
}
