//! Three garage previews retain their own CPU models and renderer allocations until eviction or close.

use std::collections::VecDeque;

use anyhow::{Result, bail};
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::{CarModel, LoadOptions};

use super::{GarageBackdrop, GarageStage, Stage};
use crate::garage::Selection;
use crate::scenes::car::CarScene;
use crate::viewer::Scene;

/// Two parked cars plus the visible car. This is bounded even while browsing many paint/part variants.
const PARKED_LIMIT: usize = 2;

#[derive(Default)]
pub(super) struct Previews {
    current: Option<Selection>,
    parked: VecDeque<Parked>,
}

struct Parked {
    key: Selection,
    car: CarScene,
    rest: (f32, f32),
    intro: f32,
}

fn matches(previous: &Selection, next: &Selection) -> bool {
    previous.car == next.car && previous.parts == next.parts && previous.color == next.color
}

impl GarageBackdrop {
    /// Restore a previously uploaded geometry selection before doing any disk read or assembly.
    pub(crate) fn restore_preview(&mut self, renderer: &mut dyn RenderBackend, selection: &Selection) -> bool {
        if self.previews.current.as_ref().is_some_and(|key| matches(key, selection)) && self.scene.car.is_some() {
            return true;
        }
        let Some(index) = self.previews.parked.iter().position(|preview| matches(&preview.key, selection)) else {
            return false;
        };
        let Parked { key, car, rest, intro } = self.previews.parked.remove(index).expect("found parked preview");
        let previous = self.scene.car.replace(car);
        self.park_preview(renderer, previous, self.scene.rest, self.scene.intro);
        (self.scene.rest, self.scene.intro) = (rest, intro);
        self.previews.current = Some(key);
        self.resume_preview(renderer);
        true
    }

    pub(crate) fn show_preview(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: &Selection,
        model: CarModel,
        options: LoadOptions,
    ) -> Result<()> {
        if self.loaded_stage.is_none() {
            self.set_stage(renderer, GarageStage::Cars)?;
        }
        // Make room before uploading, so even a replacement's peak residency is three previews.
        if self.previews.parked.len() >= PARKED_LIMIT
            && let Some(mut expired) = self.previews.parked.pop_front()
        {
            expired.car.close(renderer);
        }
        // Keep the visible preview intact if its replacement cannot be uploaded.
        let (rest, intro) = (self.scene.rest, self.scene.intro);
        let previous = self.scene.car.take();
        self.scene.upload_car(renderer, model, options);
        if self.scene.car.is_none() {
            self.scene.car = previous;
            (self.scene.rest, self.scene.intro) = (rest, intro);
            bail!("Garage preview could not be uploaded");
        }
        self.park_preview(renderer, previous, rest, intro);
        self.previews.current = Some(selection.clone());
        self.resume_preview(renderer);
        Ok(())
    }

    fn park_preview(
        &mut self,
        renderer: &mut dyn RenderBackend,
        previous: Option<CarScene>,
        rest: (f32, f32),
        intro: f32,
    ) {
        if let Some(mut previous) = previous {
            match self.previews.current.take() {
                Some(key) => self.previews.parked.push_back(Parked { key, car: previous, rest, intro }),
                None => previous.close(renderer),
            }
        }
        while self.previews.parked.len() > PARKED_LIMIT {
            if let Some(mut expired) = self.previews.parked.pop_front() {
                expired.car.close(renderer);
            }
        }
    }

    fn resume_preview(&mut self, renderer: &mut dyn RenderBackend) {
        self.scene.reflection = None;
        self.scene.still = None;
        if let Some(car) = &mut self.scene.car {
            car.set_car_shading(self.scene.shading);
            car.set_frontend_backroom(self.loaded_stage == Some(Stage::Backroom));
            car.reshow(renderer);
            // A parked car can predate a graphics change. Also restore the original backroom light rig now.
            car.refresh_effects(renderer);
        }
    }

    pub(super) fn clear_previews(&mut self, renderer: &mut dyn RenderBackend) {
        for mut preview in self.previews.parked.drain(..) {
            preview.car.close(renderer);
        }
        self.previews.current = None;
    }
}

#[cfg(test)]
#[path = "garage_preview_cache_tests.rs"]
mod tests;
