//! Authored projected road captures. This is separate from car environment cubes.
pub(super) mod geometry;
#[cfg(test)]
mod installed_tests;
mod state;
#[cfg(test)]
mod tests;
pub(in crate::scenes::world) mod vehicles;
pub(super) use state::WetRoad;

use super::{WorldScene, visibility};
use crate::settings::RoadReflections;
use blackbox_gfx::{Fog, FrameParams, ProjectedReflectionScene, RenderBackend};

impl WorldScene {
    pub(super) fn update_wet_reflections(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        let options = renderer.render_experiments();
        let active = renderer.capabilities().projected_reflections
            && options.original_world_materials
            && self.wet_road.eligible();
        if !active {
            renderer.set_projected_reflection_scene(None);
            self.wet_road.instances.clear();
            return;
        }
        let lens = self.gameplay_lens(aspect);
        // Match the playable reference's corrected projection. Stock target-aspect correction is
        // documented separately; resolution enhancements must not change the framing.
        let frame = FrameParams {
            view: lens.view,
            projection: lens.projection,
            camera_position: lens.position,
            light_dir: self.sun.direction(),
            clear_color: self.fog_colour(),
            fog: Some(Fog { start: self.fog_distance * 0.5, end: self.fog_distance }),
            scattering: self.scattering(),
            surface_light: self.surface_light(),
            camera_cut: true,
            shadow_tuning: None,
        };
        let camera = visibility::Camera {
            view_proj: frame.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: self.sun.direction().to_array(),
        };
        // Content thresholds retain the original 240-line budget when image resolution increases.
        geometry::scenery(
            self.residency.reflection_placed(),
            &camera,
            self.residency.props(),
            240.0,
            &mut self.wet_road.instances,
        );
        if options.original_world_materials {
            self.residency.sky.place(
                &self.residency.shared,
                &mut self.wet_road.instances,
                lens.position,
                super::sky::View::RoadReflection,
            );
        }
        self.append_wet_vehicles();
        self.wet_road.instances.sort_unstable_by_key(|instance| instance.mesh);
        renderer.set_projected_reflection_scene(Some(ProjectedReflectionScene {
            frame,
            instances: &self.wet_road.instances,
            size: self.wet_road.resolution.size(),
            blur: self.wet_road.quality == RoadReflections::Blurred,
        }));
    }

    fn append_wet_vehicles(&mut self) {
        if !matches!(self.wet_road.quality, RoadReflections::Vehicles | RoadReflections::Blurred) {
            return;
        }
        if let Some(drive) = &self.drive {
            drive.road_reflection_instances(&mut self.wet_road.instances);
        }
        if let Some(traffic) = &self.traffic {
            traffic.road_reflection_instances(&mut self.wet_road.instances);
        }
    }
}
