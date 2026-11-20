//! Installed car catalog validation and model preparation, executed by the preview worker.

use anyhow::{Context, Result, ensure};
use game_install::GameDir;
use nfsmw_data::car::{CarModel, LoadOptions, customization, physics::PhysicsData};

use crate::garage::{CatalogSnapshot, Model, Selection, my_cars};

#[derive(Clone)]
pub(super) struct Request {
    pub selection: Selection,
    pub slot: Option<String>,
    pub read_slot: bool,
    pub commit: bool,
    pub initial: bool,
}

impl Request {
    pub fn matches(&self, next: &Self) -> bool {
        if self.read_slot || next.read_slot {
            return self.read_slot == next.read_slot
                && self.slot == next.slot
                && self.selection.car == next.selection.car;
        }
        super::same_geometry(&self.selection, &next.selection)
    }
}

pub(super) struct Prepared {
    pub selection: Selection,
    pub catalog: CatalogSnapshot,
    pub model: CarModel,
}

pub(super) fn options() -> LoadOptions {
    LoadOptions { lod: 'A', all_parts: false, preset: None }
}

pub(super) fn prepare(
    dir: &GameDir,
    inventory: &customization::Inventory,
    physics: &PhysicsData,
    mut state: Model,
    request: Request,
) -> Result<Prepared> {
    let entry =
        state.cars.iter_mut().find(|car| car.folder == request.selection.car).context("Car is not installed")?;
    if entry.parts.is_none() {
        entry.parts = Some(inventory.catalog(dir, &entry.folder)?.into());
    }
    let selection = match request.read_slot {
        true => {
            my_cars::load(dir, &state, request.slot.as_deref().context("Missing custom slot")?, &request.selection.car)?
        }
        false => request.selection,
    };
    validate(&state, physics, &selection)?;
    let mut model = nfsmw_data::car::load_customized(dir, &selection.car, &options(), &selection.parts)?;
    if let Some(color) = selection.color {
        model.set_body_color(color)?;
    }
    let catalog = state
        .cars
        .iter()
        .find(|car| car.folder == selection.car)
        .context("Car disappeared")?
        .parts
        .as_ref()
        .unwrap()
        .clone();
    Ok(Prepared { selection, catalog, model })
}

pub(super) fn validate(state: &Model, physics: &PhysicsData, selection: &Selection) -> Result<(f32, bool)> {
    let entry = state.cars.iter().find(|car| car.folder == selection.car).context("Car is not installed")?;
    let catalog = entry.parts.as_ref().context("Parts are unavailable")?;
    catalog.validate(&selection.parts)?;
    selection.gauges.validate(catalog)?;
    selection.tuning.validate()?;
    selection.exhaust.validate()?;
    ensure!(selection.color.is_none() || entry.paintable, "This car has a fixed livery");
    let car = physics.car_with_customization(&entry.physics_name, selection.performance, &selection.parts)?;
    Ok((car.spec.engine.max_rpm, car.spec.induction.kind() != blackbox_vehicle::induction::InductionKind::None))
}
