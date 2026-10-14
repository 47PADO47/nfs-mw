//! The car's physics (`blackbox-vehicle`) seen from the game: input in, a pose to draw out, and the
//! change of axes between the library's physics space and the render space.
//!
//! Physics space is x right, y up, z forward (left-handed); the world and the car models are x forward,
//! y left, z up. The library numbers the wheels 0 front left, 1 front right, 2 rear left, 3 rear right;
//! the car model has 2 rear right, 3 rear left.

use blackbox_collision::CollisionWorld;
use blackbox_vehicle::induction::InductionKind;
use blackbox_vehicle::{FIXED_STEP, Ground, InputState, Vehicle};
use glam::{Mat3, Quat, Vec3};
use nfsmw_data::car::WheelPose;
use nfsmw_data::car::physics::{AiCollisionReactions, CarGeometry, CarPhysics, SurfaceTable, WallSpec};
use std::sync::Arc;

use super::input::DriveInput;
use super::rig::CarPose;
use super::walls::{self, ContactKind, Impact};
use crate::scenes::world::effects::{self, Contact};
use crate::scenes::world::events::{OtherBody, VehicleEvents};
use crate::scenes::world::object_geometry::{BodyGeometryFrame, BodyPose};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// What the readout shows.
#[derive(Debug, Clone, Copy, Default)]
pub struct Telemetry {
    /// Speed along the car in metres per second (negative in reverse).
    pub speed_mps: f32,
    pub rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// Nitrous tank, 0..1.
    pub nos: f32,
    /// The nitrous is burning (held, in gear, on the throttle, fast enough), not just asked for.
    pub nos_burning: bool,
    /// The car has a nitrous system.
    pub has_nos: bool,
    pub wheels_on_ground: usize,
    /// The engine's red line, rpm.
    pub red_line: f32,
    /// The engine's `MAX_RPM`: it picks the tachometer's scale.
    pub max_rpm: f32,
    /// The engine's idle speed, rpm.
    pub idle: f32,
    /// A gear change is in progress.
    pub shifting: bool,
    /// The gearbox wants the next gear (the shift light).
    pub shift_up: bool,
    /// The car has forced induction (a turbo or a supercharger).
    pub has_induction: bool,
    /// The boost gauge, psi.
    pub boost_psi: f32,
}

/// Wheel spin (m/s) the tire sound ignores: the dead zone is a fifth of it.
const SLIP_TOLERANCE: f32 = 0.5;
/// Library wheel index of each model wheel.
const PHYSICS_WHEEL: [usize; 4] = [0, 1, 3, 2];
/// Suspension travel shown on a wheel is limited to this, metres: a wheel in the air does not fly off.
const MAX_VISIBLE_TRAVEL: f32 = 0.25;

pub struct CarSim {
    vehicle: Vehicle,
    /// Where the body box's centre is relative to the model's origin, in physics axes.
    pivot: Vec3,
    /// Height of each wheel's centre above the model's origin at rest, in model wheel order.
    rest_height: [f32; 4],
    /// Roll angle of each wheel, in model wheel order.
    spin: [f32; 4],
    walls: WallSpec,
    visual_tires: Option<[super::visual_tires::VisualTire; 4]>,
    pub(in crate::scenes::world) geometry: Option<Arc<CarGeometry>>,
    pub(in crate::scenes::world) events: Option<VehicleEvents>,
    pub(in crate::scenes::world) collision_role: OtherBody,
    reactions: Option<Arc<AiCollisionReactions>>,
    /// Original goal key when an explicit AI goal is active; otherwise the installed default.
    pub(in crate::scenes::world) collision_goal: Option<u32>,
    pub(in crate::scenes::world) collision_racer: bool,
    previous_contact_pose: BodyPose,
    before_velocity: Vec3,
    ground_reference: blackbox_collision::ground_reference::GroundReferenceCache,
    wheel_ground: std::cell::RefCell<[blackbox_collision::ground_reference::GroundFaceCache; 4]>,
    /// The car touched a prop (a fixed or a movable one) this step; read once by the traffic (`take_prop_struck`).
    prop_struck: bool,
}

/// A vector from physics space to render space axes: `(x, y, z) -> (z, -x, y)`.
fn to_model_axes(v: Vec3) -> Vec3 {
    Vec3::new(v.z, -v.x, v.y)
}

impl CarSim {
    /// `rest_height` is the height of each wheel's centre above the car model's origin when the car
    /// stands still, in the model's wheel order (the static wheel placement).
    pub fn new(physics: CarPhysics, rest_height: [f32; 4]) -> Self {
        log::debug!(
            "body: half dimensions {:?}, centre {:?} above the model origin",
            physics.spec.dimension,
            physics.bounds.pivot
        );
        let mut vehicle = Vehicle::new(physics.spec);
        vehicle.set_tire_integration(physics.tire_integration);
        vehicle.jump_config = physics.jump_config;
        if physics.tire_integration == blackbox_vehicle::tires::TireIntegration::Reference {
            vehicle.wall_steering_config = nfsmw_data::car::physics::wall_steering_config();
        }
        Self {
            vehicle,
            prop_struck: false,
            pivot: physics.bounds.pivot,
            rest_height,
            spin: [0.0; 4],
            walls: physics.walls,
            visual_tires: None,
            geometry: physics.geometry,
            events: physics.events.and_then(|spec| match VehicleEvents::new(spec) {
                Ok(events) => Some(events),
                Err(error) => {
                    log::warn!("vehicle event engine unavailable: {error:#}");
                    None
                }
            }),
            collision_role: OtherBody { vehicle: true, player: true, ..Default::default() },
            reactions: physics.reactions,
            collision_goal: None,
            collision_racer: false,
            previous_contact_pose: (Vec3::ZERO, Quat::IDENTITY),
            before_velocity: Vec3::ZERO,
            ground_reference: Default::default(),
            wheel_ground: Default::default(),
        }
    }

    /// A sim for a car drawn with `rig`: the wheel heights and the tire shapes come from the model.
    pub fn for_rig(physics: CarPhysics, rig: &super::rig::CarRig) -> Self {
        let mut sim = Self::new(physics, rig.rest_heights());
        sim.set_visual_tires(rig.visual_tires());
        sim
    }

    pub(super) fn set_visual_tires(&mut self, tires: Option<[super::visual_tires::VisualTire; 4]>) {
        self.visual_tires = tires;
    }

    pub(super) fn set_rest_heights(&mut self, heights: [f32; 4]) {
        self.rest_height = heights;
    }

    /// Put the car on the road at `spawn`, standing still. False when `ground` has nothing there.
    pub fn place(&mut self, ground: &dyn Ground, spawn: Spawn) -> bool {
        self.place_moving(ground, spawn, 0.0)
    }

    /// Put the car on the road at `spawn`, already rolling forward at `speed` m/s.
    pub fn place_moving(&mut self, ground: &dyn Ground, spawn: Spawn, speed: f32) -> bool {
        self.spin = [0.0; 4];
        self.ground_reference.clear();
        for wheel in self.wheel_ground.get_mut() {
            wheel.clear();
        }
        let [x, _, z] = space::to_physics(spawn.position);
        // The heading is counter-clockwise from +X in the world; the library's yaw is about the up axis
        // from +z, which is the other way round.
        let top = spawn.position.z + 2.0;
        let placed = self.vehicle.place_on_ground_moving(ground, x, z, top, -spawn.heading, speed);
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
        placed
    }

    /// Whether the gearbox shifts by itself (the transmission setting).
    pub fn set_automatic(&mut self, automatic: bool) {
        self.vehicle.config.automatic = automatic;
    }

    /// The wheel's optional controls: the clutch pedal and the gear selector that holds a gear.
    pub fn set_wheel_options(&mut self, options: crate::settings::WheelOptions) {
        self.vehicle.config.manual_clutch = options.manual_clutch;
        self.vehicle.config.h_shifter = options.h_shifter;
    }

    /// One physics step. `world` is the collision the body's walls are tested against (the ground is
    /// `ground`); without it the car only meets the road.
    pub fn step(
        &mut self,
        input: &DriveInput,
        ground: &dyn Ground,
        mut world: Option<(&CollisionWorld, &mut PropWorld)>,
    ) -> Impact {
        self.refresh_ground_reference(world.as_ref().map(|(collision, _)| *collision));
        self.before_velocity = self.vehicle.body().linear_velocity;
        let input = InputState {
            throttle: input.throttle,
            brake: input.brake,
            steer: input.steer,
            handbrake: f32::from(u8::from(input.handbrake)),
            nos: input.nos,
            shift_up: input.shift_up,
            shift_down: input.shift_down,
            gear_select: input.gear_select,
            clutch: input.clutch,
        };
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
        if let (Some((_, props)), Some(events)) = (world.as_mut(), &self.events) {
            props.vehicle_explosions(self.vehicle.body_mut(), &events.owner, self.collision_role.player, FIXED_STEP);
        }
        if let Some(events) = &self.events {
            let punctured = events.punctured();
            for wheel in (0..4).filter(|&w| punctured[w]) {
                self.vehicle.tire_damage.puncture(wheel);
            }
        }
        self.step_with_ground(&input, ground, world.is_some());
        let impact = match world {
            Some((collision, props)) => {
                let frame = self
                    .geometry
                    .as_ref()
                    .map(|g| BodyGeometryFrame { geometry: &g.body, previous: self.previous_contact_pose });
                // The original prunes object pairs before its end-frame world collision callback.
                let mut objects = Impact::default();
                props.collide_vehicle(&mut self.vehicle, frame, &mut objects);
                let wall_exclusion = self.terrain_exclusion();
                let mut impact = walls::resolve(
                    &mut self.vehicle,
                    &walls::world_cast_with_mask(collision, wall_exclusion),
                    &walls::world_props(props),
                    &self.walls,
                );
                impact.impulse = impact.impulse.max(objects.impulse);
                impact.contacts.extend(objects.contacts);
                impact.damage.splice(0..0, objects.damage);
                impact
            }
            None => Impact::default(),
        };
        self.prop_struck |=
            impact.contacts.iter().any(|c| matches!(c.kind, ContactKind::PropRigid | ContactKind::PropMovable));
        if let Some(events) = self.events.as_mut() {
            for &hit in &impact.damage {
                events.hit(hit);
            }
        }
        for (spin, &physics) in self.spin.iter_mut().zip(&PHYSICS_WHEEL) {
            *spin += self.vehicle.wheel(physics).angular_velocity * FIXED_STEP;
        }
        impact
    }

    /// The wheels as the car sound reads them, in the model's wheel order (front left, front right, rear
    /// right, rear left).
    pub fn wheel_sounds(&self, surfaces: &SurfaceTable) -> [blackbox_carsound::WheelInput; 4] {
        let v = &self.vehicle;
        std::array::from_fn(|i| {
            let w = v.wheel(PHYSICS_WHEEL[i]);
            let audio = v.wheel_surface_tag(PHYSICS_WHEEL[i]).map(|tag| surfaces.audio(tag));
            // The tire reports how fast the patch slides over the road and how much of that is wheel spin; the
            // rest is sideways.
            let sideways = (w.slide_speed * w.slide_speed - w.slip * w.slip).max(0.0).sqrt();
            blackbox_carsound::WheelInput {
                on_ground: w.on_ground,
                slip: w.slip,
                tolerated_slip: SLIP_TOLERANCE,
                skid: sideways,
                load: w.load,
                compression: w.compression,
                traction_usage: 1.0 - w.traction,
                skid_surface: audio.map_or(0, |a| a.skid_type),
                road_noise_loop: audio.map_or(blackbox_carsound::NO_ROAD_NOISE, |a| a.road_loop),
                blown: false,
            }
        })
    }

    /// Where each tyre's ray met the road in the last step and the surface normal there (physics space), for
    /// the `debug collisions` view.
    /// The four wheels' contact patches on the road plane, in physics wheel order, for the spike strips (`spikes.rs`).
    pub fn wheel_patches(&self) -> [blackbox_pursuit::spikes::Patch; 4] {
        std::array::from_fn(|i| {
            let wheel = self.vehicle.wheel(i);
            blackbox_pursuit::spikes::Patch {
                position: glam::Vec2::new(wheel.position.x, wheel.position.z),
                radius: wheel.radius,
                on_ground: wheel.on_ground,
            }
        })
    }

    /// Punctures tire `wheel` (physics order) on the tire model. Returns whether it was intact.
    pub fn puncture_wheel(&mut self, wheel: usize) -> bool {
        self.vehicle.tire_damage.puncture(wheel)
    }

    /// Whether the car touched a prop since the last call.
    pub fn take_prop_struck(&mut self) -> bool {
        std::mem::take(&mut self.prop_struck)
    }

    pub fn tyre_hits(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        (0..4).filter_map(|i| self.vehicle.wheel_ground_hit(i))
    }

    /// The `simsurface` hash under the front left wheel, for the sounds of a landing.
    pub fn surface_tag(&self) -> Option<u32> {
        self.vehicle.wheel_surface_tag(PHYSICS_WHEEL[0])
    }

    /// Grounded tire visuals, in physics wheel order, using the existing skid/smoke intensities.
    pub fn tire_contacts(&self, collision: &CollisionWorld) -> [Option<Contact>; 4] {
        let pose = self.pose();
        std::array::from_fn(|i| {
            let mut wheel = self.vehicle.wheel(i);
            let (sin, cos) = wheel.steer_angle.sin_cos();
            let forward = self.vehicle.rotation() * Vec3::new(sin, 0.0, cos);
            // The rear-wheel permutation is its own inverse: physics order -> model order.
            let visual = self.visual_tires.map(|tires| tires[PHYSICS_WHEEL[i]]);
            if let Some(tire) = visual {
                let point = pose.transform().transform_point3(tire.point(pose.wheels[PHYSICS_WHEEL[i]]));
                wheel.position = Vec3::from_array(space::to_physics(point));
            }
            let mut contact = effects::project(wheel, forward, collision)?;
            if let Some(tire) = visual {
                contact.width = tire.width;
            }
            Some(contact)
        })
    }

    pub fn effect_velocity(&self) -> Vec3 {
        space::to_render(self.vehicle.linear_velocity().to_array())
    }

    /// The dot product of the car's up vector with the world's.
    pub fn up_dot(&self) -> f32 {
        self.vehicle.rotation().y_axis.y
    }

    /// Whether the state is usable (no NaN or runaway values). Covers the rotation, the wheels and the engine
    /// too: a NaN there freezes the car while its position still looks fine.
    pub fn is_finite(&self) -> bool {
        let v = &self.vehicle;
        let angles = v.wheel_angles();
        v.position().is_finite()
            && v.orientation().is_finite()
            && v.linear_velocity().is_finite()
            && v.angular_velocity().is_finite()
            && v.rpm().is_finite()
            && angles.left.is_finite()
            && angles.right.is_finite()
    }

    /// The model's origin in physics space.
    fn model_origin(&self) -> Vec3 {
        self.vehicle.position() - self.vehicle.rotation() * self.pivot
    }

    pub fn pose(&self) -> CarPose {
        let v = &self.vehicle;
        let r = v.rotation();
        // The model's axes (forward, left, up) are the body's z, -x and y.
        let cols = Mat3::from_cols(to_model_axes(r.z_axis), to_model_axes(-r.x_axis), to_model_axes(r.y_axis));
        let origin = self.model_origin();
        let wheels = std::array::from_fn(|i| {
            let w = v.wheel(PHYSICS_WHEEL[i]);
            let centre = w.position + r.y_axis * w.radius;
            let local = r.transpose() * (centre - origin);
            let travel = (local.y - self.rest_height[i]).clamp(-MAX_VISIBLE_TRAVEL, MAX_VISIBLE_TRAVEL);
            // The library steers to the right, the model to the left.
            WheelPose { steer: -w.steer_angle, spin: self.spin[i], travel }
        });
        CarPose { position: space::to_render(origin.to_array()), rotation: Quat::from_mat3(&cols), wheels }
    }

    /// The zones whose glass is cracked.
    pub fn cracked_zones(&self) -> Vec<usize> {
        let Some(events) = self.events.as_ref() else { return Vec::new() };
        let mut out = Vec::new();
        for zone in 0..events.damage.zone_count() {
            if events.damage.zone(zone).glass > 0 {
                out.push(zone);
            }
        }
        out
    }

    /// Apply an impact to a zone, as if the car was hit (the `damage` console command). Returns whether it changed.
    pub fn inflict(&mut self, zone: usize, severity: u8, glazed: bool) -> bool {
        let Some(events) = self.events.as_mut() else {
            log::warn!("this car has no damage events; the hit is not applied");
            return false;
        };
        let changed = events.damage.impact(zone, severity, glazed);
        log::info!("damage: zone {zone} severity {severity}, changed {changed}, now {:?}", events.damage.zone(zone));
        if usize::from(severity) >= nfsmw_data::car::damage::PUNCTURE_SEVERITY
            && let Some(wheel) = nfsmw_data::car::damage::ZONE_WHEEL.get(zone).copied().flatten()
        {
            events.damage.puncture(wheel);
        }
        changed
    }

    /// Clear the damage; the punctures follow on the next step.
    pub fn repair(&mut self) {
        if let Some(events) = self.events.as_mut() {
            events.damage.repair();
            self.vehicle.tire_damage.reset();
        }
    }

    /// Lift or restore the engine's rev limiter.
    pub fn set_no_rev_limiter(&mut self, on: bool) {
        self.vehicle.set_no_rev_limiter(on);
    }

    /// The body's half extents in the physics frame (x width, y height, z length).
    pub fn half_extents(&self) -> Vec3 {
        self.vehicle.body().dimension()
    }

    pub fn telemetry(&self) -> Telemetry {
        let v = &self.vehicle;
        Telemetry {
            speed_mps: v.forward_speed(),
            rpm: v.rpm(),
            gear: v.gear() as i32 - 1,
            nos: v.nos_level(),
            nos_burning: v.nos_burning(),
            has_nos: v.has_nos(),
            wheels_on_ground: v.wheels_on_ground(),
            red_line: v.powertrain().engine_spec().red_line,
            max_rpm: v.powertrain().engine_spec().max_rpm,
            idle: v.powertrain().engine_spec().idle,
            shifting: v.powertrain().shifting(),
            shift_up: v.powertrain().shift_up_wish(),
            has_induction: v.spec().induction.kind() != InductionKind::None,
            boost_psi: v.boost_psi(),
        }
    }
}

mod ai;
mod camera;
mod pair;
mod performance;
mod terrain;
mod trailer;

#[cfg(test)]
mod tests;
