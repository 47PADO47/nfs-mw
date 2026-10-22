//! Original body terrain cache settings; queried at the preceding tick's final body position.

#[cfg(test)]
mod tests;

use blackbox_collision::CollisionWorld;
use blackbox_collision::ground_reference::{
    GroundFace, GroundFaceCache, GroundReferenceSettings, PointQuerySettings, RegionSettings,
};
use blackbox_vehicle::ground::{Ground, GroundHit, SurfaceGrip, WheelPlane};
use blackbox_vehicle::vehicle::GroundReference;
use blackbox_vehicle::{FIXED_STEP, InputState};
use glam::Vec3;
use std::cell::RefCell;

use super::CarSim;

const SETTINGS: GroundReferenceSettings = GroundReferenceSettings {
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
    reuse_face: true,
    preserve_face: true,
};

const POINT_SETTINGS: PointQuerySettings = PointQuerySettings {
    cache: SETTINGS,
    minimum_face_offset: 0.5,
    unknown_winding_height: 1.0,
    downward_distance: 750.0,
    upright_face_exclusion: 8,
};

impl CarSim {
    pub(super) fn terrain_exclusion(&self) -> u16 {
        if self.collision_role.player || self.collision_racer {
            return 0;
        }
        if self.collision_role.cop {
            return 0x80;
        }
        0x40
    }

    pub(super) fn step_with_ground(&mut self, input: &InputState, ground: &dyn Ground, prepared: bool) {
        if !prepared {
            self.vehicle.step(FIXED_STEP, input, ground);
            return;
        }
        let ground =
            PreparedGround { fallback: ground, faces: self.ground_reference.faces(), wheels: &self.wheel_ground };
        self.vehicle.step(FIXED_STEP, input, &ground);
    }

    pub(super) fn refresh_ground_reference(&mut self, world: Option<&CollisionWorld>) {
        let Some(world) = world else {
            self.vehicle.ground_reference = None;
            return;
        };
        let radius = self.geometry.as_ref().map_or(self.half_extents().length().max(3.0), |geometry| {
            geometry.body.primitives.iter().map(|p| p.half.length() + p.centre.length()).fold(3.0, f32::max)
        });
        let settings = GroundReferenceSettings { instance_exclusion: self.terrain_exclusion(), ..SETTINGS };
        let plane = match self.vehicle.body().state {
            blackbox_vehicle::rigid_body::BodyState::Asleep => {
                // The original no-collider WWorldPos path creates WCollisionMgr(0, 3),
                // independently of the role mask used by the awake rigid-body collider.
                self.ground_reference
                    .update_global(world, self.vehicle.position().to_array(), &POINT_SETTINGS, |_| false)
            }
            _ => self.ground_reference.update(
                world,
                self.vehicle.position().to_array(),
                radius,
                &settings,
                // Matches the current free-roam adapter; dynamic group activation is not yet modeled.
                |_| false,
            ),
        };
        // The original chassis reads the retained normal/separation even after a fresh miss.
        self.vehicle.ground_reference =
            Some(GroundReference { normal: Vec3::from_array(plane.normal), separation: plane.separation });
    }
}

/// Four face histories consume the same body collider region without rebuilding it per wheel.
struct PreparedGround<'a> {
    fallback: &'a dyn Ground,
    faces: &'a [GroundFace],
    wheels: &'a RefCell<[GroundFaceCache; 4]>,
}

impl Ground for PreparedGround<'_> {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        self.fallback.hit(origin, dir, max_distance)
    }

    fn body_friction(&self, tag: u32) -> f32 {
        self.fallback.body_friction(tag)
    }

    fn world_friction(&self, tag: u32) -> f32 {
        self.fallback.world_friction(tag)
    }

    fn surface_grip(&self, tag: u32) -> SurfaceGrip {
        self.fallback.surface_grip(tag)
    }

    fn wheel_plane(&self, wheel: usize, point: Vec3, tolerance: f32, reuse: bool) -> Option<WheelPlane> {
        let settings = GroundReferenceSettings { vertical_tolerance: tolerance, reuse_face: reuse, ..SETTINGS };
        let plane = self.wheels.borrow_mut().get_mut(wheel)?.update(point.to_array(), self.faces, &settings);
        Some(WheelPlane {
            normal: Vec3::from_array(plane.normal),
            separation: plane.separation,
            surface: self.surface_grip(plane.surface_hash),
        })
    }
}
