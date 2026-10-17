//! Freeing the car's GPU resources when the scene leaves the window.

use blackbox_gfx::RenderBackend;

use super::Drive;

impl Drive {
    /// Free the car's rig and the flame textures, the loaded ones included.
    pub(in crate::scenes::world) fn release(mut self, renderer: &mut dyn RenderBackend) {
        self.flames.unload();
        self.flames.release(renderer);
        self.rig.release(renderer);
    }
}
