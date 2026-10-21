//! Original camera inputs share physics timestamps; presentation uses unscaled time.

use blackbox_collision::ground_reference::{GroundReferenceSettings, RegionSettings};

use super::{View, WorldScene, drive::SpeedbreakerClock, space};
use crate::viewer::camera::original::{self, CameraHit, Frame, PhysicsFrame};

#[cfg(test)]
mod contact_tests;
#[cfg(test)]
mod tests;

fn wall_hit(collision: &blackbox_collision::CollisionWorld, from: glam::Vec3, to: glam::Vec3) -> Option<CameraHit> {
    let hit = collision.camera_ray_cast(space::to_physics(from), space::to_physics(to), |_| false)?;
    Some(CameraHit { point: space::to_render(hit.point), normal: space::to_render(hit.normal) })
}

const GROUND: GroundReferenceSettings = GroundReferenceSettings {
    region: RegionSettings {
        initial_expansion: 1.1,
        motion_threshold: 5.0,
        prediction: 2.5,
        padding: 0.1,
        maximum_radius: 25.0,
    },
    vertical_tolerance: 0.025,
    minimum_vertical_separation: -1.0,
    maximum_normal_y: 0.9999,
    edge_length_epsilon: 1e-6,
    continuation_mask: 4,
    instance_exclusion: 0,
    maximum_cell_span: 20,
    reuse_face: false,
    preserve_face: false,
};

pub(super) struct Environment {
    pub(super) views: Option<nfsmw_data::camera::Views>,
    regions: blackbox_roads::TrackZones,
    safety_ground: std::cell::RefCell<blackbox_collision::ground_reference::GroundReferenceCache>,
}

impl Environment {
    pub(super) fn new(views: Option<nfsmw_data::camera::Views>, regions: blackbox_roads::TrackZones) -> Self {
        Self { views, regions, safety_ground: Default::default() }
    }
}

impl WorldScene {
    pub(super) fn constrain_custom_camera(
        &self,
        lens: crate::viewer::camera::Lens,
        aspect: f32,
        interpolate: bool,
    ) -> crate::viewer::camera::Lens {
        let Some(drive) = &self.drive else { return lens };
        let Some(body) = drive.original_body(interpolate) else { return lens };
        let mut bodies = Vec::new();
        bodies.extend(drive.camera_avoidable());
        if let Some(traffic) = &self.traffic {
            traffic.camera_bodies(&mut bodies);
        }
        self.residency.props().camera_bodies(&mut bodies);
        let collision = self.residency.collision();
        original::constrain_custom(
            body.position,
            lens,
            aspect,
            original::covered(&self.camera_environment.regions, lens.position, body.position),
            &bodies,
            |from, to| wall_hit(collision, from, to),
            |query, center, radius| {
                self.camera_environment.safety_ground.borrow_mut().point_height(
                    collision,
                    space::to_physics(query),
                    space::to_physics(center),
                    radius,
                    &GROUND,
                    |_| false,
                )
            },
        )
    }

    fn original_inputs(&self) -> Option<PhysicsFrame> {
        if self.view != View::Chase {
            return None;
        }
        let drive = self.drive.as_ref()?;
        let body = drive.original_body(false)?;
        let mut avoidables = Vec::new();
        let mut vehicles = Vec::new();
        // The anchored body participates in roof clearance, but never in the
        // separate under-another-vehicle predicate.
        avoidables.extend(drive.camera_avoidable());
        if let Some(traffic) = &self.traffic {
            traffic.camera_bodies(&mut avoidables);
            traffic.camera_vehicle_bounds(&mut vehicles);
        }
        self.residency.props().camera_bodies(&mut avoidables);
        Some(PhysicsFrame {
            body,
            explosions: self.residency.props().camera_explosions(),
            custom: original::index_of(self.camera_mode)
                .is_none()
                .then(|| original::custom_sample(self.car_lens(drive, drive.custom_camera(), self.physics_aspect))),
            avoidables,
            vehicles,
            noise: drive.original_noise(&self.physics.surfaces),
            chopper_positions: self
                .traffic
                .as_ref()
                .map_or_else(Vec::new, |traffic| traffic.camera_chopper_positions()),
            impacts: match self.view == View::Chase && self.original_player_step != Some(drive.original_step()) {
                true => drive.original_impacts(),
                false => Vec::new(),
            },
        })
    }

    /// Placement happens outside the physics loop. Seed it before recording ticks.
    pub(super) fn seed_original_camera(&mut self) {
        let Some(drive) = &self.drive else { return };
        let step = drive.original_step();
        if self.original_player_step.is_none() || self.original_player_step.is_some_and(|old| step < old) {
            let Some(input) = self.original_inputs() else { return };
            self.original_camera.place(input);
            self.camera_ground.clear();
            self.camera_environment.safety_ground.get_mut().clear();
        }
        self.original_player_step = Some(step);
    }

    pub(super) fn record_original_camera(&mut self) {
        let input = self.original_inputs();
        self.original_player_step = input.as_ref().and_then(|_| self.drive.as_ref().map(|drive| drive.original_step()));
        self.original_camera.physics_step(input);
    }

    pub(super) fn update_original_camera(
        &mut self,
        real_dt: f32,
        sim_dt: f32,
        ramp: SpeedbreakerClock,
        active: bool,
        simulation_scale: f32,
    ) {
        let view = original::index_of(self.camera_mode)
            .filter(|_| self.view == View::Chase)
            .and_then(|mode| self.camera_environment.views.as_ref().map(|views| (mode, &views.views[mode])));
        let frame = Frame {
            real_dt,
            sim_dt,
            view,
            custom: self.view == View::Chase && original::index_of(self.camera_mode).is_none(),
            aspect: self.physics_aspect,
            regions: Some(&self.camera_environment.regions),
            speedbreaker: active,
            look_back: self.drive.as_ref().is_some_and(|drive| drive.camera_looking_back(self.camera_mode)),
        };
        let collision = self.residency.collision();
        self.original_camera.advance(
            frame,
            |offset| simulation_offset(ramp, active, offset, simulation_scale),
            |from, to| wall_hit(collision, from, to),
            |query, center, radius| {
                self.camera_ground.point_height(
                    collision,
                    space::to_physics(query),
                    space::to_physics(center),
                    radius,
                    &GROUND,
                    // Free-roam does not currently model dynamic collision-group activation.
                    |_| false,
                )
            },
        );
    }
}

/// Match the exact scale used before physics ticks, even if a moment changes during those ticks.
fn simulation_offset(mut ramp: SpeedbreakerClock, active: bool, offset: f32, scale: f32) -> f32 {
    ramp.advance(active, offset) * scale
}
