//! The rear-view mirror's frame: the world as seen from the player's car, looking back. The player's car is
//! left out, as the mirror shows what is behind it.

use blackbox_gfx::{Fog, FrameParams, Instance};

use super::{WorldScene, visibility};
use crate::viewer::camera::{Camera, RearView};

impl WorldScene {
    /// The mirror's frame, or `None` with no car to look back from.
    pub(super) fn rear_frame(&mut self, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        let pose = self.drive.as_ref()?.pose();
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
        visibility::collect_unsorted_with_shadows(
            self.residency.placed().filter(|p| original || !p.original_shadow),
            &cull,
            rules,
            self.residency.props(),
            caster_distance,
            &mut self.rear_visible,
        );
        self.residency.props().fragment_instances(&mut self.rear_visible);
        if let Some(traffic) = &self.traffic {
            traffic.instances(&mut self.rear_visible);
        }
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
