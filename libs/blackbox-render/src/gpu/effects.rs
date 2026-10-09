//! The world effects on the renderer: `blackbox-gpu-passes` owns the buffers and pipelines, this wires
//! them to the device and exposes the layer on [`Renderer`].

use super::Renderer;
use crate::EffectLayer;

pub(super) use blackbox_gpu_passes::{Effects, SoftDraw};

impl Renderer {
    /// Replace the world effects drawn in every following scene/capture, until replaced again.
    pub fn set_effects(&mut self, layer: &EffectLayer) {
        self.effects.upload(&self.device, &self.queue, layer);
    }

    /// Allocated capacities in vertices (surface, particle); counts may fall to zero while reused.
    pub fn effect_capacities(&self) -> [usize; 2] {
        [self.effects.surface_capacity(), self.effects.particle_capacity()]
    }

    /// Allocated additive-streak capacity in vertices; retained when the streak layer is cleared.
    pub fn streak_capacity(&self) -> usize {
        self.effects.streak_capacity()
    }
}
