//! Freeing the GPU resources a residency holds when its scene leaves the window.

use blackbox_gfx::RenderBackend;

use super::{Residency, TileState};

impl Residency {
    /// Free the GPU resources of the sky, the shared sets and of every resident tile. Repeated calls are harmless:
    /// what is freed is taken out of the residency.
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.sky.release(renderer);
        self.release_normal_materials(renderer);
        std::mem::take(&mut self.shared).release(renderer);
        for (_, state) in self.tiles.drain() {
            if let TileState::Resident { resources, .. } = state {
                resources.release(renderer);
            }
        }
    }

    /// Release new registered materials on scene replacement as well as tile retirement.
    fn release_normal_materials(&mut self, renderer: &mut dyn RenderBackend) {
        self.shared.release_normal_materials(renderer);
        for tile in self.tiles.values_mut() {
            if let TileState::Resident { resources, .. } = tile {
                resources.release_normal_materials(renderer);
            }
        }
    }
}
