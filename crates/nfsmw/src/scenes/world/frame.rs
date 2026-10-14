//! Main-view selection and per-frame lighting inputs.
use super::{WorldScene, visibility};
use blackbox_gfx::{Fog, FrameParams, Instance};

impl WorldScene {
    pub(super) fn world_frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let lens = self.gameplay_lens(aspect);
        let fog_end = self.fog_distance;
        let cull = visibility::Camera {
            view_proj: lens.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: self.sun.direction().to_array(),
        };
        let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
        let original = self.original_shadows;
        visibility::collect_with_shadows(
            self.residency.placed().filter(|p| original || !p.original_shadow),
            &cull,
            rules,
            self.residency.props(),
            self.shadow_quality.sampling().2,
            &mut self.visible,
        );
        self.residency.props().fragment_instances(&mut self.visible);
        if let Some(traffic) = &self.traffic {
            traffic.instances(&mut self.visible);
            let lamps = self.lamp_meshes.as_ref().filter(|_| self.traffic_setting.lights);
            traffic.lamp_instances(lamps, lens.position, &mut self.visible);
        }
        if let Some(drive) = &self.drive {
            drive.instances(&mut self.visible);
            if let (true, Some(meshes)) = (self.markers_on, &self.marker_meshes) {
                drive.marker_instances(meshes, &mut self.visible);
            }
        }
        let drive_cut = self.drive.as_mut().is_some_and(|drive| drive.take_camera_cut());
        let camera_cut = self.cut.take() | drive_cut;
        let params = FrameParams {
            view: lens.view,
            projection: lens.projection,
            camera_position: lens.position,
            light_dir: self.sun.direction(),
            clear_color: self.fog_colour(),
            fog: Some(Fog { start: fog_end * 0.5, end: fog_end }),
            scattering: self.scattering(),
            surface_light: self.surface_light(),
            camera_cut,
        };
        (params, &self.visible)
    }
}
