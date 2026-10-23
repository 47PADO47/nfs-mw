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
        self.environment_lights.refresh(&self.residency);
        let lights = &mut self.environment_lights;
        lights.detail.configure(self.detail.settings);
        let camera = visibility::Camera {
            view_proj: lens.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: self.sun.direction().to_array(),
        };
        let candidates: Vec<_> = self
            .residency
            .placed()
            .filter(|p| lights.contains(p))
            .filter(|p| {
                let controlled = self.traffic_setting.lights
                    && p.signal.is_some_and(|head| {
                        self.traffic.as_ref().is_some_and(|traffic| traffic.controls_signal_head(head))
                    });
                !controlled
            })
            .collect();
        visibility::collect_subset(
            candidates.into_iter(),
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
