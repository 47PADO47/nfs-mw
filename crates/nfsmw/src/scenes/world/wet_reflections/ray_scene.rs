//! Reflection geometry follows resident world objects, independent of the camera frustum.
use super::super::{WorldScene, visibility};
use blackbox_gfx::{Instance, RenderBackend};
impl WorldScene {
    pub(super) fn update_ray_scene(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        if !renderer.ray_reflections_enabled() {
            return;
        }
        let lens = self.gameplay_lens(aspect);
        let options = renderer.render_experiments();
        let reach = options.road_reflection_range.max(options.ray_car_range) + 80.0;
        self.wet_road.instances.clear();
        for tile in self.residency.cull_tiles() {
            for p in tile.placed {
                if p.original_shadow || !tile.group_shown(p.group) {
                    continue;
                }
                let Some((transform, bounds)) = visibility::current_pose(p, self.residency.props()) else { continue };
                let radius = (bounds.max - bounds.min).length() * 0.5;
                if bounds.center().distance(lens.position) > reach + radius {
                    continue;
                }
                let Some(mesh) = p.lods.iter().flatten().next() else { continue };
                self.wet_road
                    .instances
                    .push(Instance::keyed(*mesh, transform, p.key).with_flipped_winding(p.flip_winding()));
            }
        }
        // Vehicle geometry is merged from the actual main draw, keeping wheel/damage transforms.
        renderer.set_ray_reflection_instances(&self.wet_road.instances);
    }
}
