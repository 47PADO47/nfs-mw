//! Freeing the GPU resources the world uploaded, when the scene leaves the window (`Scene::close`).

use blackbox_gfx::RenderBackend;

use super::WorldScene;

impl WorldScene {
    /// Frees what the scene uploaded: the sections, the sky and the normal materials, the car, the traffic, the
    /// contact markers, the traffic lights, the cop health bars and the explosions. The shadow casters, the reflections
    /// and the projected wet-road reflection it set are cleared; all are set again every frame, so a next scene that
    /// already runs is not affected. The focus mask is not cleared: this runs after the next scene's init, which may
    /// have set its own.
    pub(super) fn release_gpu(&mut self, renderer: &mut dyn RenderBackend) {
        renderer.set_shadow_casters(None);
        self.shadow_feed.fed = false;
        renderer.set_reflection_scene(None);
        renderer.set_projected_reflection_scene(None);
        self.wet_road.instances.clear();
        self.residency.release(renderer);
        if let Some(drive) = self.drive.take() {
            drive.release(renderer);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.release(renderer);
        }
        if let Some(markers) = self.marker_meshes.take() {
            markers.release(renderer);
        }
        if let Some(lamps) = self.lamp_meshes.take() {
            lamps.release(renderer);
        }
        self.cop_health.release(renderer);
        self.explosions.release(renderer);
        self.heli_wash.release(renderer);
    }
}

#[cfg(test)]
#[path = "teardown_tests.rs"]
mod tests;
