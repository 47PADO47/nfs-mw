//! Rebuild shared render resources without replacing live actor or physics state.

use super::{TrafficModel, TrafficWorld};
use crate::scenes::world::props::PropWorld;
use crate::settings::CarShading;
use blackbox_gfx::RenderBackend;
use glam::Vec3;

impl TrafficWorld {
    pub fn reshade(&mut self, renderer: &mut dyn RenderBackend, mode: CarShading, sun: Vec3, props: &mut PropWorld) {
        if self.shading == Some(mode) {
            return;
        }
        for model in &self.models {
            reshade_model(model, renderer, mode, sun, props);
        }
        for model in &self.cop_models {
            reshade_model(&model.model, renderer, mode, sun, props);
        }
        if let Some(rig) = &self.helicopter_rig {
            props.remap_fragment_meshes(&rig.reshade(renderer, mode, sun));
        }
        self.shading = Some(mode);
    }
}

fn reshade_model(
    model: &TrafficModel,
    renderer: &mut dyn RenderBackend,
    mode: CarShading,
    sun: Vec3,
    props: &mut PropWorld,
) {
    props.remap_fragment_meshes(&model.rig.reshade(renderer, mode, sun));
    if let Some(trailer) = &model.trailer {
        reshade_model(trailer, renderer, mode, sun, props);
    }
}
