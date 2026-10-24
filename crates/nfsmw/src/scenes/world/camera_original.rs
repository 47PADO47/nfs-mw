//! Original camera inputs share physics timestamps; presentation uses unscaled time.

use blackbox_collision::ground_reference::{GroundReferenceSettings, RegionSettings};

use super::scenery_groups::SceneryGroups;
use super::{View, WorldScene, drive::SpeedbreakerClock, space};
use crate::viewer::camera::Lens;
use crate::viewer::camera::original::{self, Avoidable, CameraHit, Frame, PhysicsFrame};

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
    /// The last custom-camera clearance and everything it was computed from.
    memo: std::cell::RefCell<Option<ClearanceMemo>>,
    /// The avoidable bodies of the call in progress, kept between calls so no call allocates them.
    bodies: std::cell::RefCell<Vec<Avoidable>>,
}

impl Environment {
    pub(super) fn new(views: Option<nfsmw_data::camera::Views>, regions: blackbox_roads::TrackZones) -> Self {
        Self { views, regions, safety_ground: Default::default(), memo: Default::default(), bodies: Default::default() }
    }
}

/// A custom-camera clearance and every input it read: the lens asked for, the car, the aspect, the bodies to avoid
/// and the scenery the walls and the ground came from (the residency's `visual_revision` and the switched-on groups).
/// The same inputs always give the same lens, so a repeat call within a frame or a tick costs one comparison.
struct ClearanceMemo {
    lens: Lens,
    car: glam::Vec3,
    aspect: u32,
    revision: u64,
    groups: SceneryGroups,
    bodies: Vec<Avoidable>,
    out: Lens,
}

impl ClearanceMemo {
    fn answers(
        &self,
        lens: &Lens,
        car: glam::Vec3,
        aspect: f32,
        revision: u64,
        groups: &SceneryGroups,
        bodies: &[Avoidable],
    ) -> bool {
        let same = |a: &Avoidable, b: &Avoidable| {
            a.transform == b.transform && a.half_extents == b.half_extents && a.velocity == b.velocity
        };
        self.lens == *lens
            && self.car == car
            && self.aspect == aspect.to_bits()
            && self.revision == revision
            && self.bodies.len() == bodies.len()
            && self.bodies.iter().zip(bodies).all(|(a, b)| same(a, b))
            && self.groups == *groups
    }
}

impl WorldScene {
    pub(super) fn constrain_custom_camera(&self, lens: Lens, aspect: f32, interpolate: bool) -> Lens {
        let Some(drive) = self.players.lead() else { return lens };
        let Some(body) = drive.original_body(interpolate) else { return lens };
        let environment = &self.camera_environment;
        let mut bodies = environment.bodies.take();
        bodies.clear();
        bodies.extend(drive.camera_avoidable());
        if let Some(traffic) = &self.traffic {
            traffic.camera_bodies(&mut bodies);
        }
        self.residency.props().camera_bodies(&mut bodies);
        let revision = self.residency.visual_revision();
        let remembered = environment.memo.borrow();
        let hit = remembered
            .as_ref()
            .filter(|memo| memo.answers(&lens, body.position, aspect, revision, &self.scenery_groups, &bodies));
        if let Some(memo) = hit {
            let out = memo.out;
            drop(remembered);
            *environment.bodies.borrow_mut() = bodies;
            return out;
        }
        drop(remembered);
        let collision = self.residency.collision();
        let out = original::constrain_custom(
            body.position,
            lens,
            aspect,
            original::covered(&environment.regions, lens.position, body.position),
            &bodies,
            |from, to| wall_hit(collision, from, to),
            |query, center, radius| {
                environment.safety_ground.borrow_mut().point_height(
                    collision,
                    space::to_physics(query),
                    space::to_physics(center),
                    radius,
                    &GROUND,
                    |_| false,
                )
            },
        );
        // The previous answer's groups are kept when they still match (cloning them is the costly part), and its
        // body list becomes the spare for the next call.
        let (groups, spare) = match environment.memo.take() {
            Some(old) if old.groups == self.scenery_groups => (old.groups, old.bodies),
            Some(old) => (self.scenery_groups.clone(), old.bodies),
            None => (self.scenery_groups.clone(), Vec::new()),
        };
        *environment.bodies.borrow_mut() = spare;
        let memo = ClearanceMemo { lens, car: body.position, aspect: aspect.to_bits(), revision, groups, bodies, out };
        *environment.memo.borrow_mut() = Some(memo);
        out
    }

    fn original_inputs(&self) -> Option<PhysicsFrame> {
        if self.view != View::Chase {
            return None;
        }
        let drive = self.players.lead()?;
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
        let Some(drive) = self.players.lead() else { return };
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
        self.original_player_step = input.as_ref().and_then(|_| self.players.lead().map(|drive| drive.original_step()));
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
            look_back: self.players.lead().is_some_and(|drive| drive.camera_looking_back(self.camera_mode)),
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
