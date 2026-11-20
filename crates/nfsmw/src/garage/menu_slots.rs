//! The shop rows of a vehicle with components: its lists, the rules that refuse a choice and the words of the refusal come from
//! the vehicle's options (`garage/options`, ADR 0014), not from the shop's fixed categories.
//!
//! A row of a model category (`Row::Part`) or of a level (`Row::Performance`) keeps its place in the selection, so saves, prices and
//! the original screens are as they were; for a vehicle that has components the labels of its choices are the components' names
//! and a choice is made through [`VehicleOptions::apply`], which checks the rules of the vehicle on the result. A slot that only the
//! components know (`Row::Slot`, kept in `Selection::slots`) goes the same way. A car without components never gets here.

use super::menu_rows::Row;
use super::options::{Change, Edit, Slot, VehicleOptions};
use super::{Model, Selection};

/// The slot of `options` that `row` is the shop's view of.
pub(super) fn slot_of(options: &VehicleOptions, row: Row) -> Option<&Slot> {
    match row {
        Row::Part(category) => options.of_category(category),
        Row::Performance(category) => options.of_level(category),
        Row::Slot(at) => options.slots.get(usize::from(at)),
        _ => None,
    }
}

/// The choices of `row` and the one installed, by the components' names; `None` when the row is the shop's own (a car without
/// components, or a row no slot stands behind).
pub(super) fn options(model: &Model, row: Row) -> Option<(Vec<String>, usize)> {
    let options = VehicleOptions::of_draft(model)?;
    let slot = slot_of(&options, row)?;
    Some((slot.choices.iter().map(|choice| choice.label.clone()).collect(), slot.current))
}

/// Makes the choice at `index` of `row`: `Some` when the row is a slot of the vehicle's components (the result says whether the
/// choice was made: a rule of the vehicle can refuse it, then nothing changes), `None` when the row is the shop's own.
pub(super) fn set(model: &mut Model, row: Row, index: usize) -> Option<Result<String, String>> {
    let options = VehicleOptions::of_draft(model)?;
    let slot = slot_of(&options, row)?;
    let catalog = model.selected().and_then(|car| car.parts.clone());
    let mut draft = model.draft.clone();
    let mut edit = Edit { parts: &mut draft.parts, color: &mut draft.color, performance: &mut draft.performance };
    let change = Change::Pick { slot: slot.id.clone(), index };
    let result = options.apply(catalog.as_deref(), &mut edit, &change);
    if result.is_ok() {
        model.draft = draft;
    }
    Some(result)
}

/// Why each choice of `row` cannot be fitted with what the cart holds: the vehicle's own rule, in its words (`None` for a choice
/// that can be, and for every choice of a row no component slot stands behind). Asked once for the whole list.
pub(crate) fn blocked_all(model: &Model, row: Row) -> Vec<Option<String>> {
    let Some(options) = VehicleOptions::of_draft(model) else { return Vec::new() };
    let Some(slot) = slot_of(&options, row) else { return Vec::new() };
    let say = |choice: &super::options::Choice| {
        choice.blocked.as_ref().map(|reason| format!("{} cannot be fitted: {reason}", choice.label))
    };
    slot.choices.iter().map(say).collect()
}

/// What the shop calls the slot behind `row`: the slot's own name.
pub(crate) fn label(model: &Model, row: Row) -> Option<String> {
    let options = VehicleOptions::of_draft(model)?;
    slot_of(&options, row).filter(|slot| slot.components).map(|slot| slot.label.clone())
}

/// Whether the slot behind `row` has anything to choose (a choice besides the stock one).
pub(super) fn has_choices(model: &Model, row: Row) -> Option<bool> {
    let options = VehicleOptions::of_draft(model)?;
    slot_of(&options, row).map(|slot| slot.choices.len() > 1)
}

/// The slot behind `row` and the component `selected` holds in it, by name (`Exhaust`, `Akrapovic Slip-on`): the cart's words.
/// `None` for a row no component slot stands behind.
pub(crate) fn describe(model: &Model, row: Row, selected: &Selection) -> Option<(String, String)> {
    let car = model.cars.iter().find(|car| car.folder == selected.car)?;
    let options = VehicleOptions::of_selection(car, selected)?;
    let slot = slot_of(&options, row).filter(|slot| slot.components)?;
    Some((slot.label.clone(), slot.installed().label.clone()))
}
