//! The rear-view mirror's frame: the world as seen from the player's car, looking back. The player's car is
//! left out, as the mirror shows what is behind it.

use blackbox_gfx::reflection::distance::within;
use blackbox_gfx::{Fog, FrameParams, Instance};
use glam::Vec3;

use super::{WorldScene, visibility};
use crate::viewer::camera::{Camera, RearView};

impl WorldScene {
    /// The mirror's frame, or `None` with no car to look back from.
    pub(super) fn rear_frame(&mut self, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        let pose = self.players.lead()?.pose();
        let lens = RearView { position: pose.position, rotation: pose.rotation }.lens(aspect);
        let cull = visibility::Camera {
            view_proj: lens.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: self.sun.direction().to_array(),
        };
        let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
        self.rear_visible.clear();
        let original = self.original_shadows;
        let caster_distance = match self.shadow_feed.fed {
            true => 0.0,
            false => self.shadow_quality.sampling().2,
        };
        // Apply the mirror's far plane to world bounds while scanning. The capture bridge must not
        // repeat this with model origins: world-space road meshes can have their origin at zero.
        let far = self.detail.mirror_far;
        visibility::collect_rear(
            self.residency.cull_tiles(),
            original,
            &cull,
            rules,
            self.residency.props(),
            caster_distance,
            far,
            &mut self.rear_visible,
        );
        let scenery = self.rear_visible.len();
        self.residency.props().fragment_instances(&mut self.rear_visible);
        self.mesh_animations.place(&self.residency, &mut self.rear_visible);
        self.residency.animate_simple_models(&mut self.rear_visible, self.clock);
        if self.original_sky {
            self.residency.sky.place(
                &self.residency.shared,
                &mut self.rear_visible,
                lens.position,
                super::sky::View::Mirror,
            );
        }
        if let Some(traffic) = &self.traffic {
            traffic.instances(&mut self.rear_visible);
        }
        // The sources after the scenery are few; their origins are cut here to the same limit.
        limit_tail(&mut self.rear_visible, scenery, lens.position, far);
        // Every source has appended now: one sort puts the copies of each mesh together.
        visibility::sort_by_mesh(&mut self.rear_visible);
        let fog_end = self.fog_distance;
        let params = FrameParams {
            view: lens.view,
            projection: lens.projection,
            camera_position: lens.position,
            light_dir: self.sun.direction(),
            clear_color: self.fog_colour(),
            fog: Some(Fog { start: fog_end * 0.5, end: fog_end }),
            scattering: self.scattering(),
            surface_light: self.surface_light(),
            camera_cut: true,
            shadow_tuning: None,
        };
        Some((params, &self.rear_visible))
    }
}

/// Drops the instances from `from` on whose origin is farther than `far` from `camera`, keeping the order of the rest.
fn limit_tail(list: &mut Vec<Instance>, from: usize, camera: Vec3, far: Option<f32>) {
    if far.is_none() {
        return;
    }
    let mut kept = from;
    for i in from..list.len() {
        if !within(camera, list[i].transform.w_axis.truncate(), far) {
            continue;
        }
        list.swap(kept, i);
        kept += 1;
    }
    list.truncate(kept);
}
