//! My Cars previews without loading a world. Assembly uses the same install readers as driving.

use anyhow::{Context, Result, ensure};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use game_install::GameDir;
use nfsmw_data::car::{LoadOptions, customization, physics::PhysicsData};

use super::scene::garage::{GarageBackdrop, GarageStage};
use crate::garage::{Model, Selection};
use crate::input::ActionState;
use crate::settings::CarShading;
use crate::viewer::Scene;

struct Loaded {
    model: Model,
    physics: PhysicsData,
    inventory: customization::Inventory,
}

pub(super) struct GarageScene {
    dir: GameDir,
    model: Model,
    baseline: Selection,
    baseline_slot: Option<String>,
    preview_car: String,
    physics: Option<PhysicsData>,
    inventory: Option<customization::Inventory>,
    backdrop: GarageBackdrop,
    worker: Option<std::thread::JoinHandle<Result<Loaded>>>,
    shading: CarShading,
    save: bool,
}

impl GarageScene {
    pub fn new(dir: GameDir, save: bool) -> Self {
        Self {
            backdrop: GarageBackdrop::new(dir.clone()),
            dir,
            model: Model::default(),
            baseline: Selection::default(),
            baseline_slot: None,
            preview_car: String::new(),
            physics: None,
            inventory: None,
            worker: None,
            shading: CarShading::default(),
            save,
        }
    }

    pub(super) fn with_rig(mut self, rig: super::CameraRig) -> Self {
        self.backdrop = self.backdrop.with_rig(rig);
        self
    }

    fn discover(&mut self, folder: &str) -> Result<()> {
        let entry = self.model.cars.iter_mut().find(|car| car.folder == folder).context("Car is not installed")?;
        if entry.parts.is_none() {
            entry.parts =
                Some(self.inventory.as_ref().context("Part inventory is loading")?.catalog(&self.dir, folder)?.into());
        }
        Ok(())
    }

    fn show_selection(&mut self, renderer: &mut dyn RenderBackend, selection: &Selection) -> Result<()> {
        self.discover(&selection.car)?;
        let entry = self.model.cars.iter().find(|car| car.folder == selection.car).context("Car is not installed")?;
        let catalog = entry.parts.as_ref().context("Parts are unavailable")?;
        catalog.validate(&selection.parts)?;
        selection.gauges.validate(catalog)?;
        selection.tuning.validate()?;
        let physics = self
            .physics
            .as_ref()
            .context("Physics is loading")?
            .car_with_performance(&entry.physics_name, selection.performance)?;
        ensure!(selection.color.is_none() || entry.paintable, "This car has a fixed livery");
        let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
        let mut model = nfsmw_data::car::load_customized(&self.dir, &selection.car, &options, &selection.parts)?;
        if let Some(color) = selection.color {
            model.set_body_color(color)?;
        }
        self.backdrop.set_car_shading(self.shading);
        self.backdrop.show_model(renderer, model, options)?;
        self.model.gauge_max_rpm = physics.spec.engine.max_rpm;
        self.model.gauge_has_turbo = physics.spec.induction.kind() != blackbox_vehicle::induction::InductionKind::None;
        self.preview_car = selection.car.clone();
        Ok(())
    }

    fn settle(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        if self.worker.as_ref().is_none_or(|worker| !worker.is_finished()) {
            return Ok(());
        }
        let loaded = self
            .worker
            .take()
            .expect("finished worker")
            .join()
            .map_err(|_| anyhow::anyhow!("Garage loader panicked"))??;
        self.model = loaded.model;
        self.physics = Some(loaded.physics);
        self.inventory = Some(loaded.inventory);
        self.model.custom_slots_enabled = true;
        self.gate_cars();
        // A new slot starts on an available car with upgrade packages, rather than a fixed hero build.
        if let Some(car) = self.model.cars.iter().find(|car| {
            car.unavailable.is_none()
                && car.paintable
                && car.performance.categories.iter().any(|part| part.max_level > 0)
        }) {
            self.model.applied = Selection { car: car.folder.clone(), ..Default::default() };
        }
        self.refresh_slots()?;
        self.model.reset_draft();
        let selection = self.model.draft.clone();
        self.discover(&selection.car)?;
        self.show_selection(renderer, &selection)?;
        self.model.applied = selection.clone();
        self.model.draft = selection.clone();
        self.baseline = selection;
        self.refresh_unlock_error(&self.model.draft.clone());
        if let Some((id, _)) = self.model.custom_slots.first().cloned()
            && let Err(error) = self.select_slot(renderer, &id)
        {
            self.model.message = Some(Err(format!("Saved custom car could not be loaded: {error:#}")));
        }
        Ok(())
    }
}

#[path = "garage_slots.rs"]
mod slots;
#[path = "garage_unlocks.rs"]
mod unlocks;
pub(crate) use unlocks::validate_selection;

impl Scene for GarageScene {
    fn title(&self) -> String {
        "nfsmw — My Cars".into()
    }

    fn init(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        let dir = self.dir.clone();
        self.worker = Some(std::thread::Builder::new().name("my-cars-load".into()).spawn(move || {
            let physics = PhysicsData::load(&dir)?;
            let mut model = crate::scenes::world::garage_catalog(&dir, &physics)?;
            model.cars.retain(|car| car.category == crate::garage::Category::Racing);
            ensure!(!model.cars.is_empty(), "No installed racing cars are available");
            let inventory = customization::Inventory::load(&dir)?;
            Ok(Loaded { model, physics, inventory })
        })?);
        Ok(())
    }

    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.backdrop.close(renderer);
        self.worker = None;
    }

    fn ready(&self) -> bool {
        self.worker.is_none()
    }

    fn set_car_shading(&mut self, shading: CarShading) {
        self.shading = shading;
        self.backdrop.set_car_shading(shading);
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        if let Err(error) = self.settle(renderer) {
            self.model.message = Some(Err(format!("{error:#}")));
            self.worker = None;
        }
        self.backdrop.update(renderer, input, dt);
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        self.backdrop.frame(aspect)
    }

    fn prepare_frame(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        self.backdrop.prepare_frame(renderer, aspect);
    }

    fn reattached(&mut self, renderer: &mut dyn RenderBackend) {
        self.backdrop.reattached(renderer);
    }

    fn refresh_effects(&mut self, renderer: &mut dyn RenderBackend) {
        self.backdrop.refresh_effects(renderer);
    }

    fn garage(&mut self) -> Option<&mut Model> {
        Some(&mut self.model)
    }

    fn garage_view(
        &mut self,
        renderer: &mut dyn RenderBackend,
        stage: crate::garage::View,
        camera: &'static str,
    ) -> Result<()> {
        let stage = match stage {
            crate::garage::View::Cars => GarageStage::Cars,
            crate::garage::View::Customization => GarageStage::Customization,
            crate::garage::View::Backroom => GarageStage::Backroom,
        };
        self.backdrop.set_stage(renderer, stage)?;
        self.backdrop.show_camera(camera);
        Ok(())
    }

    fn begin_garage(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        self.baseline = self.model.applied.clone();
        self.baseline_slot = self.model.custom_slot.clone();
        let message = self.model.message.take();
        self.model.reset_draft();
        self.model.message = message;
        Ok(())
    }

    fn preview_garage(&mut self, renderer: &mut dyn RenderBackend, selection: Selection) -> Result<()> {
        if selection.car != self.preview_car {
            self.model.custom_slot = None;
        }
        self.show_selection(renderer, &selection)?;
        self.refresh_unlock_error(&selection);
        self.model.draft = selection;
        Ok(())
    }

    fn apply_garage(&mut self, renderer: &mut dyn RenderBackend, selection: Selection) -> Result<()> {
        self.show_selection(renderer, &selection)?;
        self.model.applied = selection.clone();
        self.model.draft = selection;
        Ok(())
    }

    fn confirm_garage(&mut self, renderer: &mut dyn RenderBackend, selection: Selection) -> Result<()> {
        self.validate_unlocks(&selection)?;
        self.show_selection(renderer, &selection)?;
        if self.save {
            self.persist_slot(&selection)?;
        }
        self.model.applied = selection.clone();
        self.model.draft = selection.clone();
        self.baseline = selection;
        self.baseline_slot = self.model.custom_slot.clone();
        Ok(())
    }

    fn cancel_garage(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let baseline = self.baseline.clone();
        if baseline.car.is_empty() {
            return Ok(());
        }
        self.show_selection(renderer, &baseline)?;
        self.model.draft = baseline;
        self.model.custom_slot = self.baseline_slot.clone();
        Ok(())
    }

    fn select_garage_slot(&mut self, renderer: &mut dyn RenderBackend, id: &str) -> Result<()> {
        self.select_slot(renderer, id)
    }

    fn add_garage_slot(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.add_slot(renderer)
    }

    fn delete_garage_slot(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.delete_slot(renderer)
    }

    fn radio_on_menus(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "garage_scene_tests.rs"]
mod tests;
