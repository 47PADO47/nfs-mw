//! Main-view selection and per-frame lighting inputs.
use super::{View, WorldScene, visibility};
use crate::settings::CameraMode;
use crate::viewer::camera::original;
use blackbox_gfx::{Fog, FrameParams, Instance};

impl WorldScene {
    pub(super) fn world_frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let _span = bevy_log::info_span!("world::frame").entered();
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
        if let Some(traffic) = &mut self.traffic {
            let placed = self.residency.placed().filter_map(super::ai::signal_props::candidate);
            traffic.update_signal_props(self.residency.visual_revision(), placed);
            traffic.instances(&mut self.visible);
            let lamps = self.lamp_meshes.as_ref().filter(|_| self.traffic_setting.lights);
            traffic.lamp_instances(lamps, lens.position, self.residency.props(), &mut self.visible);
        }
        if let Some(drive) = &self.drive {
            if render_player(self.view, self.camera_mode, drive.camera_looking_back(self.camera_mode)) {
                drive.instances(&mut self.visible);
            }
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
            shadow_tuning: None,
        };
        (params, &self.visible)
    }
}

fn render_player(view: View, mode: CameraMode, look_back: bool) -> bool {
    view != View::Chase || original::renders_car(mode, look_back)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_body_gates_apply_only_to_the_driving_main_view() {
        assert!(!render_player(View::Chase, CameraMode::Bumper, false));
        assert!(!render_player(View::Chase, CameraMode::Bumper, true));
        assert!(render_player(View::Chase, CameraMode::Hood, false));
        assert!(!render_player(View::Chase, CameraMode::Hood, true));
        for mode in CameraMode::ALL {
            for look_back in [false, true] {
                assert!(render_player(View::Fly, mode, look_back));
            }
        }
    }

    #[test]
    fn outside_and_custom_cameras_keep_the_player_visible() {
        for mode in [
            CameraMode::Chase,
            CameraMode::DynamicFov,
            CameraMode::Close,
            CameraMode::Far,
            CameraMode::SuperFar,
            CameraMode::Drift,
            CameraMode::Pursuit,
        ] {
            assert!(render_player(View::Chase, mode, false));
            assert!(render_player(View::Chase, mode, true));
        }
    }
}
