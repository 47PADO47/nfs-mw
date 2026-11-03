//! Initial metadata is loaded separately so the native My Cars list can open before its preview finishes.

use super::*;

pub(super) fn load(dir: &GameDir) -> Result<Loaded> {
    let physics = Arc::new(PhysicsData::load(dir)?);
    let mut model = crate::scenes::world::garage_catalog(dir, &physics)?;
    model.cars.retain(|car| car.category == crate::garage::Category::Racing);
    ensure!(!model.cars.is_empty(), "No installed racing cars are available");
    let inventory = Arc::new(customization::Inventory::load(dir)?);
    // Warm the shared reader off-thread; preview validation retains its existing error reporting.
    let _ = nfsmw_data::car::shop_unlocks::load(dir);
    unlocks::gate_cars(dir, &mut model);
    model.custom_slots_enabled = true;
    slots::read_slots(&mut model)?;
    if let Some(car) = model.cars.iter().find(|car| {
        car.unavailable.is_none() && car.paintable && car.performance.categories.iter().any(|part| part.max_level > 0)
    }) {
        model.applied = Selection { car: car.folder.clone(), ..Default::default() };
    }
    model.reset_draft();
    Ok(Loaded { model, physics, inventory })
}

impl GarageScene {
    pub(super) fn settle(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        if self.worker.as_ref().is_none_or(|worker| !worker.is_finished()) {
            return Ok(());
        }
        let loaded = self.worker.take().unwrap().join().map_err(|_| anyhow::anyhow!("Garage loader panicked"))??;
        self.model = loaded.model;
        self.physics = Some(loaded.physics);
        self.inventory = Some(loaded.inventory);
        let fallback = self.model.draft.clone();
        self.initial_fallback = Some(fallback.clone());
        self.baseline = fallback.clone();
        if let Some((id, _)) = self.model.custom_slots.first().cloned() {
            match self.select_slot(renderer, &id) {
                Ok(()) => return Ok(()),
                Err(error) => self.initial_warning = Some(format!("Saved custom car could not be loaded: {error:#}")),
            }
        }
        self.show_selection(renderer, &fallback)
    }
}
