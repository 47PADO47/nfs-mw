//! Select the same scenery LODs, group visibility and live poses as the main view.

use blackbox_gfx::{RenderBackend, TexturedEffect};

use super::super::{WorldScene, visibility};

impl WorldScene {
    pub(in crate::scenes::world) fn environment_effects(
        &mut self,
        renderer: &dyn RenderBackend,
    ) -> Vec<TexturedEffect> {
        let lens = self.gameplay_lens(renderer.aspect_ratio());
        if let Some(traffic) = &mut self.traffic {
            traffic.update_signal_props(
                self.residency.visual_revision(),
                self.residency.placed().filter_map(super::super::ai::signal_props::candidate),
            );
        }
        self.environment_lights.refresh(&self.residency, &self.scenery_groups);
        let lights = &mut self.environment_lights;
        lights.detail.configure(self.detail.settings);
        let camera = visibility::Camera {
            view_proj: lens.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: self.sun.direction().to_array(),
        };
        // The marked placements were picked at `refresh`; what changes between frames is only which scenery groups
        // are in and which signal heads a junction controller has taken over.
        let collision = self.residency.collision();
        let signals = self.traffic_setting.lights;
        let traffic = self.traffic.as_ref();
        let controlled = |head| signals && traffic.is_some_and(|traffic| traffic.controls_signal_head(head));
        let candidates = lights
            .candidates
            .iter()
            .filter(|p| p.group == 0 || collision.group_enabled(p.group))
            .filter(|p| !p.signal.is_some_and(controlled));
        visibility::collect_subset(
            candidates,
            &camera,
            &blackbox_scenery::layout::MOST_WANTED.lod,
            self.residency.props(),
            &self.scenery_groups,
            0.0,
            &mut lights.detail,
            &mut lights.visible,
        );
        self.residency.props().fragment_instances(&mut lights.visible);
        let mut effects = Vec::new();
        lights.draw(
            &self.residency.shared.materials,
            super::super::car_light_flares::FlareView {
                camera: lens.position,
                forward: lens.forward,
                fov_degrees: lens.fov_y_degrees,
                height: renderer.surface_size()[1],
            },
            &mut effects,
        );
        effects
    }
}
