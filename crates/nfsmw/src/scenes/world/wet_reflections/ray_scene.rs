//! Reflection geometry follows resident world objects, independent of the camera frustum.
use super::super::{WorldScene, visibility};
use blackbox_gfx::{Instance, RenderBackend};
use glam::{Mat4, Vec3, Vec4};
impl WorldScene {
    pub(super) fn update_ray_scene(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        if !renderer.ray_reflections_enabled() {
            return;
        }
        let lens = self.gameplay_lens(aspect);
        let options = renderer.render_experiments();
        let sway = blackbox_scenery::sway::SwayFrame {
            phase: blackbox_scenery::layout::MOST_WANTED.sway.phase(self.clock),
            axis: self.wind.axis(0, lens.position, lens.forward).to_array(),
        };
        // Sway only what the main view sways (in view, or a nearby off-camera caster), so it merges with the main
        // draw. Off-screen scenery stays at rest: its transform stays bit-identical and keeps the TLAS reusable.
        let planes = planes_of(&lens.view_proj());
        let caster_distance = match self.shadow_feed.fed {
            true => 0.0,
            false => self.shadow_quality.sampling().2,
        };
        let reach = options.road_reflection_range.max(options.ray_car_range) + 80.0;
        self.wet_road.instances.clear();
        for tile in self.residency.cull_tiles() {
            for p in tile.placed {
                if p.original_shadow || !tile.group_shown(p.group) {
                    continue;
                }
                let Some((mut transform, bounds)) = visibility::current_pose(p, self.residency.props()) else {
                    continue;
                };
                let radius = (bounds.max - bounds.min).length() * 0.5;
                let distance = bounds.center().distance(lens.position);
                if distance > reach + radius {
                    continue;
                }
                let Some((slot, mesh)) = p.lods.iter().enumerate().find_map(|(slot, mesh)| Some((slot, (*mesh)?)))
                else {
                    continue;
                };
                let swaying = p.sway_lods & (1 << slot) != 0 && p.prop_id == visibility::NOT_A_PROP;
                let main_sways = sphere_in_view(&planes, bounds.center(), radius)
                    || (caster_distance > 0.0 && distance <= caster_distance + 80.0 + radius);
                if swaying && main_sways {
                    transform = visibility::swayed(transform, &sway);
                }
                self.wet_road
                    .instances
                    .push(Instance::keyed(mesh, transform, p.key).with_flipped_winding(p.flip_winding()));
            }
        }
        // Vehicle geometry is merged from the actual main draw, keeping wheel/damage transforms.
        renderer.set_ray_reflection_instances(&self.wet_road.instances);
    }
}

/// The six planes of `view_proj` (`n · p + d >= 0` inside), as the main view's cull makes them.
fn planes_of(view_proj: &Mat4) -> [Vec4; 6] {
    let rows = [view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3)];
    let normalize = |p: Vec4| p / p.truncate().length().max(f32::EPSILON);
    [
        normalize(rows[3] + rows[0]),
        normalize(rows[3] - rows[0]),
        normalize(rows[3] + rows[1]),
        normalize(rows[3] - rows[1]),
        normalize(rows[2]),
        normalize(rows[3] - rows[2]),
    ]
}

fn sphere_in_view(planes: &[Vec4; 6], center: Vec3, radius: f32) -> bool {
    planes.iter().all(|plane| plane.truncate().dot(center) + plane.w >= -radius)
}
