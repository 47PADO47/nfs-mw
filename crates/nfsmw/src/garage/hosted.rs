//! What the vehicle panel (F11) hosts from the garage: the pages that need the garage's model of the driven car, drawn with the
//! garage's own controls and applied the way the garage applies a change (`docs/devtools-vehicle.md`).

use super::{Model, Selection};
use crate::settings::Settings;

/// Starts a frame of the hosted pages: the draft is what is applied now. `None` while a normal shop transaction owns the model
/// (the cart is its draft), when the tools stay out of it. Returns the draft to compare with at the end.
pub(crate) fn begin(model: &mut Model) -> Option<Selection> {
    if model.shop {
        return None;
    }
    model.draft = model.applied.clone();
    Some(model.draft.clone())
}

/// The selection to apply when a hosted page changed the draft since [`begin`].
pub(crate) fn changed(model: &Model, before: &Selection) -> Option<Selection> {
    (model.draft != *before && model.selected().is_none_or(|car| car.unavailable.is_none()))
        .then(|| model.draft.clone())
}

/// The saved setups of the car: save, load, update.
pub(crate) fn setups(ui: &mut egui::Ui, model: &mut Model) {
    ui.collapsing("Saved setups", |ui| super::presets::show(ui, model));
}

/// The tuning profiles and their sliders.
pub(crate) fn tuning(ui: &mut egui::Ui, model: &mut Model) {
    super::tuning::show(ui, model);
}

/// The vinyl colours, the rim colour, the gauges and the licence plate.
pub(crate) fn appearance(ui: &mut egui::Ui, model: &mut Model, settings: &mut Settings) {
    super::appearance::extras(ui, model, settings);
}
