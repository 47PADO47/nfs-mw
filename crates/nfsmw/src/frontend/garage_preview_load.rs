//! Queue CPU work without blocking menu input; only the newest requested geometry may reach the renderer.

use anyhow::Context;

use super::*;
use crate::garage::PreviewUpdate;
use decode::{Prepared, Request};

impl GarageScene {
    pub(super) fn show_selection(&mut self, renderer: &mut dyn RenderBackend, selection: &Selection) -> Result<()> {
        self.request_preview(
            renderer,
            Request {
                selection: selection.clone(),
                slot: None,
                read_slot: false,
                commit: false,
                initial: self.initial_fallback.is_some(),
            },
        )
    }

    pub(super) fn request_preview(&mut self, renderer: &mut dyn RenderBackend, request: Request) -> Result<()> {
        let entry =
            self.model.cars.iter().find(|car| car.folder == request.selection.car).context("Car is not installed")?;
        ensure!(request.selection.color.is_none() || entry.paintable, "This car has a fixed livery");
        if !request.read_slot && entry.parts.is_some() {
            decode::validate(&self.model, self.physics.as_ref().context("Physics is loading")?, &request.selection)?;
            self.backdrop.set_car_shading(self.shading);
            if self.backdrop.restore_preview(renderer, &request.selection) {
                self.previews.cancel();
                self.finish_preview(renderer, request, None)?;
                return Ok(());
            }
        }
        if !self.model.preview_loading {
            self.rollback = Some((self.model.draft.clone(), self.model.custom_slot.clone()));
        }
        self.model.draft = request.selection.clone();
        self.model.preview_loading = true;
        self.model.preview_update = PreviewUpdate::None;
        self.model.message = None;
        // The original keeps the shown car while a part, paint or tuning of the same car is prepared, and only an
        // empty room stands between two different cars (R2: no car/shadow flash on each customization item).
        let other_car = self.rendered.as_ref().is_none_or(|shown| shown.car != request.selection.car);
        self.backdrop.preview_pending(other_car);
        self.previews.request(request);
        if let Err(error) = self.start_preview() {
            let message = format!("{error:#}");
            self.fail_preview(renderer, error);
            anyhow::bail!("{message}");
        }
        Ok(())
    }

    fn start_preview(&mut self) -> Result<()> {
        if !self.previews.needs_start() {
            return Ok(());
        }
        let Some(inventory) = self.inventory.clone() else { return Ok(()) };
        let Some(physics) = self.physics.clone() else { return Ok(()) };
        let (dir, model) = (self.dir.clone(), self.model.clone());
        self.previews.start(move |request| decode::prepare(&dir, &inventory, &physics, model, request))
    }

    pub(super) fn poll_preview(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some((request, result)) = self.previews.poll(Request::matches) {
            let result = result.and_then(|prepared| self.finish_preview(renderer, request.clone(), Some(prepared)));
            if let Err(error) = result {
                if request.initial && request.slot.is_some() {
                    self.initial_warning = Some(format!("Saved custom car could not be loaded: {error:#}"));
                    if let Some(fallback) = self.initial_fallback.clone() {
                        if let Err(error) = self.show_selection(renderer, &fallback) {
                            self.fail_preview(renderer, error);
                        }
                        return;
                    }
                }
                self.fail_preview(renderer, error);
            }
        }
        if let Err(error) = self.start_preview() {
            self.fail_preview(renderer, error);
        }
    }

    fn finish_preview(
        &mut self,
        renderer: &mut dyn RenderBackend,
        request: Request,
        prepared: Option<Prepared>,
    ) -> Result<()> {
        let mut selection = request.selection;
        if let Some(prepared) = prepared {
            let entry = self.model.cars.iter_mut().find(|car| car.folder == prepared.selection.car).unwrap();
            entry.parts = Some(prepared.catalog);
            if request.read_slot {
                selection = prepared.selection;
            }
            decode::validate(&self.model, self.physics.as_ref().context("Physics is loading")?, &selection)?;
            self.backdrop.set_car_shading(self.shading);
            self.backdrop.show_preview(renderer, &selection, prepared.model, decode::options())?;
        }
        let (rpm, turbo) =
            decode::validate(&self.model, self.physics.as_ref().context("Physics is loading")?, &selection)?;
        self.model.gauge_max_rpm = rpm;
        self.model.gauge_has_turbo = turbo;
        self.preview_car = selection.car.clone();
        self.rendered = Some(selection.clone());
        self.model.draft = selection.clone();
        self.model.preview_update = PreviewUpdate::None;
        if request.commit || request.initial {
            self.model.applied = selection.clone();
            self.baseline = selection.clone();
            self.model.custom_slot = request.slot;
            self.baseline_slot = self.model.custom_slot.clone();
            self.model.preview_update = PreviewUpdate::SlotReady;
        }
        self.model.preview_loading = false;
        self.backdrop.preview_pending(false);
        self.rollback = None;
        self.initial_fallback = None;
        self.refresh_unlock_error(&selection);
        self.model.message = self.initial_warning.take().map(Err);
        Ok(())
    }

    pub(super) fn fail_preview(&mut self, _renderer: &mut dyn RenderBackend, error: anyhow::Error) {
        self.previews.cancel();
        self.model.preview_loading = false;
        self.backdrop.preview_pending(false);
        if let Some((selection, slot)) = self.rollback.take() {
            self.model.draft = selection.clone();
            self.model.custom_slot = slot;
            self.refresh_unlock_error(&selection);
        }
        self.model.preview_update = PreviewUpdate::Failed;
        self.initial_fallback = None;
        self.initial_warning = None;
        self.model.message = Some(Err(format!("{error:#}")));
    }

    pub(super) fn ensure_preview_ready(&self) -> Result<()> {
        ensure!(!self.model.preview_loading, "{}", crate::i18n::text("garage-preview-wait", &crate::i18n::Args::new()));
        Ok(())
    }
}
