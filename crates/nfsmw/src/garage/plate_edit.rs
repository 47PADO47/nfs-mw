//! Changing the licence plate of a mod car outside the shop: the F6 diagnostic garage and the `plate` / `plate-design` console
//! commands (`scenes/world/garage/plate_command.rs`).
//!
//! Both only offer a plate to a car that has one: a mod car whose manifest has a `[customization.plate]`
//! (`Catalog::plate`), and designs to one that also lists some (`Catalog::plate_designs`). They change the draft selection,
//! which the world applies at once, and keep it for the session only, as everything the F6 garage does: the typed text and the
//! picked design reach `plates.toml` only through the normal garage's License Plate and Plate Design pages. The checks are the
//! garage prompt's: the characters the car's glyph sheet draws, upper case, no more than the plate shows.

use nfsmw_data::car::customization::{Catalog, PlateDesignChoice};
use nfsmw_data::car::mods::custom::{self, Plate};

use super::{Model, plates};

/// The name of the plate without a design.
pub(crate) const PLAIN: &str = "Plain plate";

/// The plate the car in `folder` can change: `None` for a car without one (a stock car, or a mod car with no plate table).
pub(crate) fn supported(catalog: &Catalog, folder: &str) -> Option<&'static Plate> {
    custom::of(folder)?.plate.as_ref().filter(|_| catalog.plate)
}

/// Text typed on the command line, checked the way the garage prompt checks the keys: every character must be one the sheet
/// draws (upper case or lower, through the aliases) and no more than the plate shows. Returns the registration to keep.
pub(crate) fn checked(plate: &Plate, text: &str) -> Result<String, String> {
    let slots = plate.slots as usize;
    if text.chars().count() > slots {
        return Err(format!("the plate shows at most {slots} characters"));
    }
    if let Some(bad) = text.chars().find(|c| plate.glyph(*c).is_none()) {
        return Err(format!("the plate has no {bad:?} character"));
    }
    Ok(plate.registration(text))
}

/// Text typed in a field, as the garage prompt would take it key by key: the drawable characters only, up to the slots.
pub(crate) fn typed(plate: &Plate, text: &str) -> String {
    text.chars().filter_map(|c| plate.glyph(c)).take(plate.slots as usize).collect()
}

/// The text to keep for typed `text`: a text of spaces only is no text, and the car wears its own again (`None`).
pub(crate) fn chosen(plate: &Plate, text: &str) -> Option<String> {
    let text = plate.registration(text);
    (!text.trim().is_empty()).then_some(text)
}

/// The text the car in `folder` wears: the one `picked` for the session, else the one the player kept, else the manifest's.
pub(crate) fn worn(plate: &Plate, folder: &str, picked: Option<&str>) -> String {
    let kept = plates::text_for(folder);
    plate.worn(picked.or(kept.as_deref())).map(|text| plate.registration(text)).unwrap_or_default()
}

/// The name of the design the car wears, [`PLAIN`] for none: the one `picked` for the session, else the player's, else the
/// manifest's.
pub(crate) fn design_name<'a>(plate: &Plate, catalog: &'a Catalog, folder: &str, picked: Option<&str>) -> &'a str {
    let kept = plates::design_for(folder);
    let Some(id) = plate.design_worn(picked, kept.as_deref()) else { return PLAIN };
    catalog.plate_designs.iter().find(|design| design.id == id).map_or(PLAIN, |design| design.name.as_str())
}

/// The id a design is picked by: `none` (or `plain`) for the plain plate, else an id of the list, else a name of it (case
/// ignored).
pub(crate) fn design_id(designs: &[PlateDesignChoice], wanted: &str) -> Result<String, String> {
    if wanted.eq_ignore_ascii_case("none") || wanted.eq_ignore_ascii_case("plain") {
        return Ok(String::new());
    }
    let found = designs
        .iter()
        .find(|design| design.id == wanted)
        .or_else(|| designs.iter().find(|design| design.name.eq_ignore_ascii_case(wanted)));
    match found {
        Some(design) => Ok(design.id.clone()),
        None => Err(format!("no plate design {wanted:?} (plate-design list shows them)")),
    }
}

/// The F6 garage's plate row: a text field and a design chooser, only for a car with a plate (the design chooser only when the
/// plate offers designs). The text applies when the field loses focus (Enter, or a click elsewhere), so the car is not built
/// again at every key.
pub(super) fn show(ui: &mut egui::Ui, model: &mut Model, catalog: &Catalog) {
    let folder = model.draft.car.clone();
    let Some(plate) = supported(catalog, &folder) else { return };
    ui.collapsing("License plate", |ui| {
        text_row(ui, model, plate, &folder);
        if !catalog.plate_designs.is_empty() {
            design_row(ui, model, catalog, plate, &folder);
        }
        let own = model.draft.parts.plate.is_none() && model.draft.parts.plate_design.is_none();
        if ui.add_enabled(!own, egui::Button::new("Restore the car's own plate")).clicked() {
            model.draft.parts.plate = None;
            model.draft.parts.plate_design = None;
        }
        ui.weak("This session only: the garage's License Plate page keeps a text for the car.");
    });
}

fn text_row(ui: &mut egui::Ui, model: &mut Model, plate: &Plate, folder: &str) {
    let id = egui::Id::new(("garage_plate_text", folder));
    let now = worn(plate, folder, model.draft.parts.plate.as_deref());
    let mut text = ui.data_mut(|data| data.get_temp::<String>(id)).unwrap_or_else(|| now.clone());
    let response = ui
        .horizontal(|ui| {
            ui.label("Text");
            ui.add(egui::TextEdit::singleline(&mut text).desired_width(110.0).char_limit(plate.slots as usize))
        })
        .inner;
    text = typed(plate, &text);
    if response.has_focus() {
        ui.data_mut(|data| data.insert_temp(id, text));
        return;
    }
    ui.data_mut(|data| data.remove::<String>(id));
    if response.lost_focus() && plate.registration(&text) != now {
        model.draft.parts.plate = chosen(plate, &text);
    }
}

fn design_row(ui: &mut egui::Ui, model: &mut Model, catalog: &Catalog, plate: &Plate, folder: &str) {
    let picked = model.draft.parts.plate_design.clone();
    let shown = design_name(plate, catalog, folder, picked.as_deref());
    ui.horizontal(|ui| {
        ui.label("Design");
        egui::ComboBox::from_id_salt(("garage_plate_design", folder)).selected_text(shown).show_ui(ui, |ui| {
            if ui.selectable_label(shown == PLAIN, PLAIN).clicked() {
                model.draft.parts.plate_design = Some(String::new());
            }
            for design in &catalog.plate_designs {
                if ui.selectable_label(shown == design.name, &design.name).clicked() {
                    model.draft.parts.plate_design = Some(design.id.clone());
                }
            }
        });
    });
}

#[cfg(test)]
#[path = "plate_edit_tests.rs"]
mod tests;
