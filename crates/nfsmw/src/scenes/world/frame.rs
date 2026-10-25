//! Main-view selection and per-frame lighting inputs.
use super::{View, WorldScene};
use crate::settings::CameraMode;
use crate::viewer::camera::original;
use blackbox_gfx::{Fog, FrameParams, Instance};

impl WorldScene {
    pub(super) fn world_frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let _trace = blackbox_trace::scope("World: build render frame");
        let _span = bevy_log::info_span!("world::frame").entered();
        self.drawn_aspect = aspect;
        let lens = self.gameplay_lens(aspect);
        let fog_end = self.fog_distance;
        let mut visible = std::mem::take(&mut self.visible);
        self.fill_view(&lens, super::split::Viewer::First, &mut visible);
        self.visible = visible;
        let drive_cut = self.players.lead_mut().is_some_and(|drive| drive.take_camera_cut());
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

impl WorldScene {
    /// Whether the first player's own car is in their picture: the win photo shows it, and the camera mode decides
    /// otherwise (a hood or bumper view is inside it).
    pub(super) fn player_in_picture(&self, drive: &super::drive::Drive) -> bool {
        self.extra.freecam
            || self.extra.selected_bookmark.is_some()
            || self.career.garage.is_some()
            || self.photo_shot.is_some()
            || render_player(self.view, self.camera_mode, drive.camera_looking_back(self.camera_mode))
    }
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
