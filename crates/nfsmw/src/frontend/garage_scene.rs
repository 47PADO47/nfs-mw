//! My Cars previews without loading a world. Assembly uses the same install readers as driving.

use anyhow::{Result, ensure};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use game_install::GameDir;
use nfsmw_data::car::{customization, physics::PhysicsData};
use std::sync::Arc;

use super::scene::garage::{GarageBackdrop, GarageStage};
use crate::garage::{Model, Selection};
use crate::input::ActionState;
use crate::settings::CarShading;
use crate::viewer::Scene;

struct Loaded {
    model: Model,
    physics: Arc<PhysicsData>,
    inventory: Arc<customization::Inventory>,
}

pub(super) struct GarageScene {
    dir: GameDir,
    model: Model,
    baseline: Selection,
    baseline_slot: Option<String>,
    preview_car: String,
    rendered: Option<Selection>,
    physics: Option<Arc<PhysicsData>>,
    inventory: Option<Arc<customization::Inventory>>,
    backdrop: GarageBackdrop,
    worker: Option<std::thread::JoinHandle<Result<Loaded>>>,
    shading: CarShading,
    save: bool,
    previews: jobs::Jobs<decode::Request, decode::Prepared>,
    rollback: Option<(Selection, Option<String>)>,
    initial_fallback: Option<Selection>,
    initial_warning: Option<String>,
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
            rendered: None,
            physics: None,
            inventory: None,
            worker: None,
            shading: CarShading::default(),
            save,
            previews: jobs::Jobs::default(),
            rollback: None,
            initial_fallback: None,
            initial_warning: None,
        }
    }

    pub(super) fn with_rig(mut self, rig: super::CameraRig) -> Self {
        self.backdrop = self.backdrop.with_rig(rig);
        self
    }
}

#[path = "garage_decode.rs"]
mod decode;
#[path = "garage_preview_jobs.rs"]
mod jobs;
#[path = "garage_loading.rs"]
mod loading;
#[path = "garage_preview_load.rs"]
mod preview_load;
#[path = "garage_slots.rs"]
mod slots;
#[path = "garage_unlocks.rs"]
mod unlocks;
pub(crate) use unlocks::validate_selection;

impl Scene for GarageScene {
    fn title(&self) -> String {
        "nfsmw — My Cars".into()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        // The original keeps the crib and its camera up while My Cars reads its list: the room comes up at once and the
        // camera eases to the manager's pose; only the car waits for the catalogue.
        match self.backdrop.set_stage(renderer, GarageStage::Cars) {
            Ok(()) => self.backdrop.show_camera(crate::frontend::garage_ui::Page::Manager.camera()),
            Err(error) => log::warn!("My Cars room could not be shown before its catalogue: {error:#}"),
        }
        let dir = self.dir.clone();
        self.model.preview_loading = true;
        self.worker = Some(std::thread::Builder::new().name("my-cars-load".into()).spawn(move || loading::load(&dir))?);
        Ok(())
    }

    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.backdrop.close(renderer);
        self.worker = None;
        self.previews = jobs::Jobs::default();
        self.model.preview_loading = false;
        self.rollback = None;
        self.initial_fallback = None;
        self.initial_warning = None;
        self.rendered = None;
    }

    fn ready(&self) -> bool {
        self.worker.is_none() && !self.model.preview_loading
    }

    fn set_car_shading(&mut self, shading: CarShading) {
        self.shading = shading;
        self.backdrop.set_car_shading(shading);
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        if let Err(error) = self.settle(renderer) {
            self.worker = None;
            self.fail_preview(renderer, error);
        }
        self.poll_preview(renderer);
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
        ensure!(
            self.rendered.is_some() || (!self.model.cars.is_empty() && self.model.preview_loading),
            "{}",
            self.model
                .message
                .as_ref()
                .and_then(|message| message.as_ref().err())
                .map(String::as_str)
                .unwrap_or("Garage preview is unavailable")
        );
        self.baseline = self.model.applied.clone();
        self.baseline_slot = self.model.custom_slot.clone();
        if self.model.preview_loading {
            return Ok(());
        }
        let message = self.model.message.take();
        self.model.reset_draft();
        self.model.message = message;
        Ok(())
    }

    fn preview_garage(&mut self, renderer: &mut dyn RenderBackend, selection: Selection) -> Result<()> {
        let different_car = selection.car != self.preview_car;
        self.show_selection(renderer, &selection)?;
        if different_car {
            self.model.custom_slot = None;
        }
        self.refresh_unlock_error(&selection);
        self.model.draft = selection;
        Ok(())
    }

    fn apply_garage(&mut self, renderer: &mut dyn RenderBackend, selection: Selection) -> Result<()> {
        self.request_preview(
            renderer,
            decode::Request {
                selection,
                slot: self.model.custom_slot.clone(),
                read_slot: false,
                commit: true,
                initial: false,
            },
        )
    }

    fn confirm_garage(&mut self, renderer: &mut dyn RenderBackend, selection: Selection) -> Result<()> {
        self.ensure_preview_ready()?;
        self.validate_unlocks(&selection)?;
        self.show_selection(renderer, &selection)?;
        self.ensure_preview_ready()?;
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
        self.previews.cancel();
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

#[cfg(test)]
#[path = "garage_initial_tests.rs"]
mod initial_tests;

#[cfg(test)]
#[path = "garage_async_tests.rs"]
mod async_tests;

fn same_geometry(previous: &Selection, next: &Selection) -> bool {
    previous.car == next.car && previous.parts == next.parts && previous.color == next.color
}
