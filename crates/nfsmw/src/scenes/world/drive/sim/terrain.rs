//! Original body terrain cache settings; queried at the preceding tick's final body position.

#[cfg(test)]
mod tests;

use blackbox_collision::ground_reference::{
    GroundFace, GroundFaceCache, GroundReferenceCache, GroundReferenceSettings, PointQuerySettings, RegionSettings,
};
use blackbox_collision::{CandidateSet, CollisionWorld};
use blackbox_vehicle::ground::{Ground, GroundHit, SurfaceGrip, WheelPlane};
use blackbox_vehicle::rigid_body::BodyState;
use blackbox_vehicle::vehicle::GroundReference;
use blackbox_vehicle::{InputState, Vehicle};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;
use std::cell::RefCell;

use super::CarSim;
use super::step::StepStart;
use crate::scenes::world::drive::ground::{WindowedGround, WorldGround};

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

    /// The terrain query of this step, taken at the start of the step: an explosion later in the step can wake a
    /// sleeping body, and a sleeping body's query is another one.
    pub(super) fn terrain_query(&self) -> TerrainQuery {
        let radius = self.geometry.as_ref().map_or(self.half_extents().length().max(3.0), |geometry| {
            geometry.body.primitives.iter().map(|p| p.half.length() + p.centre.length()).fold(3.0, f32::max)
        });
        TerrainQuery {
            position: self.vehicle.position().to_array(),
            asleep: matches!(self.vehicle.body().state, BodyState::Asleep),
            radius,
            exclusion: self.terrain_exclusion(),
        }
    }

    /// Refreshes the terrain plane now, as a step's integration does.
    #[cfg(test)]
    fn refresh_ground_reference(&mut self, world: Option<&CollisionWorld>) {
        let terrain = world.map(|world| (world, self.terrain_query()));
        refresh_ground_reference(&mut self.vehicle, &mut self.ground_reference, terrain);
    }

    /// The part of the step that touches only the body, its terrain caches and the static collision.
    pub(in crate::scenes::world) fn integration(&mut self) -> Integration<'_> {
        Integration {
            vehicle: &mut self.vehicle,
            ground_reference: &mut self.ground_reference,
            wheel_ground: &mut self.wheel_ground,
            window: &mut self.ground_window,
        }
    }
}

/// Where the body looks for its terrain in a step (`CarSim::terrain_query`).
#[derive(Debug, Clone, Copy)]
pub(super) struct TerrainQuery {
    position: [f32; 3],
    asleep: bool,
    radius: f32,
    exclusion: u16,
}

/// The integration of one step: the body against the ground, reading only the car's own body and terrain caches and
/// the static collision. It holds nothing else of the car, so the cars' integrations can run on several threads at
/// once (`ai/parallel.rs`).
pub(in crate::scenes::world) struct Integration<'a> {
    vehicle: &'a mut Vehicle,
    ground_reference: &'a mut GroundReferenceCache,
    wheel_ground: &'a mut RefCell<[GroundFaceCache; 4]>,
    /// The car's own candidate window for [`Self::run_windowed`].
    window: &'a mut CandidateSet,
}

impl Integration<'_> {
    /// Refreshes the terrain plane and integrates the body over `ground`, as the start of the step left them.
    pub(in crate::scenes::world) fn run(
        &mut self,
        start: &StepStart,
        ground: &dyn Ground,
        world: Option<&CollisionWorld>,
    ) {
        refresh_ground_reference(self.vehicle, self.ground_reference, world.zip(start.terrain));
        step_with_ground(
            self.vehicle,
            self.ground_reference,
            self.wheel_ground,
            &start.input,
            start.delta,
            ground,
            start.prepared(),
        );
    }

    /// [`Self::run`] over the world's ground cast through the car's own window, for a car integrated beside others.
    /// The hits, and so the step, are the same.
    pub(in crate::scenes::world) fn run_windowed(
        &mut self,
        start: &StepStart,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
        refresh_ground_reference(self.vehicle, self.ground_reference, Some(collision).zip(start.terrain));
        let ground =
            WindowedGround { world: WorldGround { collision, surfaces }, window: RefCell::new(&mut *self.window) };
        step_with_ground(
            self.vehicle,
            self.ground_reference,
            self.wheel_ground,
            &start.input,
            start.delta,
            &ground,
            start.prepared(),
        );
    }
}

fn step_with_ground(
    vehicle: &mut Vehicle,
    ground_reference: &GroundReferenceCache,
    wheel_ground: &RefCell<[GroundFaceCache; 4]>,
    input: &InputState,
    dt: f32,
    ground: &dyn Ground,
    prepared: bool,
) {
    if !prepared {
        vehicle.step(dt, input, ground);
        return;
    }
    let ground = PreparedGround { fallback: ground, faces: ground_reference.faces(), wheels: wheel_ground };
    vehicle.step(dt, input, &ground);
}

/// The body's terrain plane from `terrain`'s world and query; none without a world.
fn refresh_ground_reference(
    vehicle: &mut Vehicle,
    cache: &mut GroundReferenceCache,
    terrain: Option<(&CollisionWorld, TerrainQuery)>,
) {
    let Some((world, query)) = terrain else {
        vehicle.ground_reference = None;
        return;
    };
    let settings = GroundReferenceSettings { instance_exclusion: query.exclusion, ..SETTINGS };
    let plane = match query.asleep {
        // The original no-collider WWorldPos path creates WCollisionMgr(0, 3),
        // independently of the role mask used by the awake rigid-body collider.
        true => cache.update_global(world, query.position, &POINT_SETTINGS, |_| false),
        false => cache.update(
            world,
            query.position,
            query.radius,
            &settings,
            // Matches the current free-roam adapter; dynamic group activation is not yet modeled.
            |_| false,
        ),
    };
    // The original chassis reads the retained normal/separation even after a fresh miss.
    vehicle.ground_reference =
        Some(GroundReference { normal: Vec3::from_array(plane.normal), separation: plane.separation });
}

/// Four face histories consume the same body collider region without rebuilding it per wheel.
struct PreparedGround<'a> {
    fallback: &'a dyn Ground,
    faces: &'a [GroundFace],
    wheels: &'a RefCell<[GroundFaceCache; 4]>,
}

impl Ground for PreparedGround<'_> {
    /// Rays (the body's corner rays) go to the world cast, never to `faces`. The prepared region is not the cast's
    /// candidate set: it keeps faces by distance from the body under its own exclusions and cell cap, leaves out the
    /// ungridded packs and the scenery-group rule, and keeps faces without the instance rotation; and the cast tests
    /// a ray in each instance's own space, so no world-space face list can reproduce its bits. The cast's candidate
    /// window keeps the cast's own per-instance and per-strip values between a car's rays instead, which is exact.
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
