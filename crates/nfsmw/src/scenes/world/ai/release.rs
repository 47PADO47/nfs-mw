//! Freeing the traffic's GPU resources: the rigs and light bars of the car models, and the police helicopter's rig.

use std::rc::Rc;

use blackbox_gfx::RenderBackend;

use super::cop_lights::CopLightSet;
use super::{TrafficModel, TrafficWorld};
use crate::scenes::world::drive::CarRig;

impl TrafficWorld {
    /// Free what the traffic uploaded. The cars, the police and the window pass hold clones of the shared rigs and
    /// light bars, so they are let go first; each rig and light set left is then freed by its one holder.
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.cars.clear();
        self.police.set_rig(None);
        self.window_requests.get_mut().clear();
        for rig in self.take_roadblock_rigs() {
            release_rig(rig, renderer);
        }
        for model in self.models.drain(..).chain(self.cop_models.drain(..).map(|cop| cop.model)) {
            release_model(model, renderer);
        }
        if let Some(rig) = self.helicopter_rig.take() {
            release_rig(rig, renderer);
        }
    }
}

fn release_model(model: TrafficModel, renderer: &mut dyn RenderBackend) {
    let TrafficModel { rig, trailer, cop_lights, .. } = model;
    release_rig(rig, renderer);
    if let Some(trailer) = trailer {
        release_model(*trailer, renderer);
    }
    if let Some(set) = cop_lights {
        release_light_set(set, renderer);
    }
}

/// Frees a rig nothing else holds. One still held elsewhere is logged and left alone.
fn release_rig(rig: Rc<CarRig>, renderer: &mut dyn RenderBackend) {
    match Rc::try_unwrap(rig) {
        Ok(rig) => rig.release(renderer),
        Err(_) => log::warn!("a traffic car rig is still held; its GPU resources are not freed"),
    }
}

/// Frees a light set nothing else holds. One still held elsewhere is logged and left alone.
fn release_light_set(set: Rc<CopLightSet>, renderer: &mut dyn RenderBackend) {
    match Rc::try_unwrap(set) {
        Ok(set) => set.release(renderer),
        Err(_) => log::warn!("a cop light set is still held; its GPU resources are not freed"),
    }
}
