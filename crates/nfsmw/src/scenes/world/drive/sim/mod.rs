//! The car's physics (`blackbox-vehicle`) seen from the game: input in, a pose to draw out, and the
//! change of axes between the library's physics space and the render space.
//!
//! Physics space is x right, y up, z forward (left-handed); the world and the car models are x forward,
//! y left, z up. The library numbers the wheels 0 front left, 1 front right, 2 rear left, 3 rear right;
//! the car model has 2 rear right, 3 rear left.

use blackbox_collision::CollisionWorld;
mod audio;
mod cinematic;
mod hits;
pub(in crate::scenes::world) mod memo;
pub(in crate::scenes::world) mod pair_contact;
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub(in crate::scenes::world) mod predict;
mod reset_collision;
use blackbox_vehicle::induction::InductionKind;
use blackbox_vehicle::{FIXED_STEP, Ground, InputState, Vehicle};
use glam::{Mat3, Quat, Vec3};
use nfsmw_data::car::WheelPose;
use nfsmw_data::car::physics::{AiCollisionReactions, CarGeometry, CarPhysics, SurfaceTable, WallSpec};
use std::sync::Arc;

use super::input::DriveInput;
use super::rig::CarPose;
use super::walls::{self, ContactKind, Impact, WallContact};
use crate::scenes::world::effects::{self, Contact};
use crate::scenes::world::events::{OtherBody, VehicleEvents};
use crate::scenes::world::object_geometry::{BodyGeometryFrame, BodyPose};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

mod telemetry;
pub use telemetry::Telemetry;

/// Wheel spin (m/s) the tire sound ignores: the dead zone is a fifth of it.
const SLIP_TOLERANCE: f32 = 0.5;
/// Library wheel index of each model wheel.
const PHYSICS_WHEEL: [usize; 4] = [0, 1, 3, 2];
/// Suspension travel shown on a wheel is limited to this, metres: a wheel in the air does not fly off.
const MAX_VISIBLE_TRAVEL: f32 = 0.25;

pub struct CarSim {
    vehicle: Vehicle,
    cinematic_controls: Option<blackbox_vehicle::ControlConfig>,
    reset_collision: reset_collision::ResetCollision,
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
    /// Simulation seconds of the last step, including its shared-world responses.
    step_delta: f32,
    /// Uninterrupted NPC regular-brake time for the original FX admission task.
    brake_lamp_age: f32,
    ground_reference: blackbox_collision::ground_reference::GroundReferenceCache,
    wheel_ground: std::cell::RefCell<[blackbox_collision::ground_reference::GroundFaceCache; 4]>,
    /// The candidate window of the wheels' ground rays when the car is integrated beside others (`terrain.rs`).
    ground_window: blackbox_collision::CandidateSet,
    /// The car touched a prop (a fixed or a movable one) this step; read once by the traffic (`take_prop_struck`).
    prop_struck: bool,
    /// The props the car hit with their cost to state, until the traffic takes them (`take_smacked`).
    smacked: Vec<(u32, u32)>,
    pending_sound_contacts: Vec<crate::audio::CarContact>,
    effect_source: u32,
    effect_owner: u64,
    effect_lifetime: std::rc::Rc<()>,
    pending_effects: Vec<crate::scenes::world::impact_effects::Contact>,
    sound_identity: u64,
    /// The wall contacts of the last step, kept so the step does not allocate (`walls::resolve_into`).
    wall_scratch: Vec<WallContact>,
    /// Body-phase contacts retained until the response phase publishes them.
    pending_body_impact: Impact,
    /// What the last step did to the car, for the police radio (`memo.rs`).
    memo: memo::StepMemo,
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
            cinematic_controls: None,
            reset_collision: Default::default(),
            pending_body_impact: Impact::default(),
            prop_struck: false,
            smacked: Vec::new(),
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
            step_delta: FIXED_STEP,
            brake_lamp_age: 0.0,
            ground_reference: Default::default(),
            wheel_ground: Default::default(),
            ground_window: Default::default(),
            pending_sound_contacts: Vec::new(),
            effect_source: physics.effect_source,
            effect_owner: crate::scenes::world::impact_effects::next_owner(),
            effect_lifetime: std::rc::Rc::new(()),
            pending_effects: Vec::new(),
            sound_identity: next_sound_identity(),
            wall_scratch: Vec::new(),
            memo: memo::StepMemo::default(),
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
    #[cfg(test)]
    pub fn place(&mut self, ground: &dyn Ground, spawn: Spawn) -> bool {
        self.place_moving(ground, spawn, 0.0)
    }

    /// Put the car on the road at `spawn`, already rolling forward at `speed` m/s.
    pub fn place_moving(&mut self, ground: &dyn Ground, spawn: Spawn, speed: f32) -> bool {
        self.spin = [0.0; 4];
        self.brake_lamp_age = 0.0;
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

    pub fn take_sound_contacts(&mut self) -> Vec<crate::audio::CarContact> {
        std::mem::take(&mut self.pending_sound_contacts)
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
            let sideways = w.lateral_speed;
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
            // The axle is the tire's lateral direction: the body's right turned by the steering, seen from above.
            let (sin, cos) = wheel.steer_angle.sin_cos();
            let axle = self.vehicle.rotation() * Vec3::new(cos, 0.0, -sin);
            blackbox_pursuit::spikes::Patch {
                position: glam::Vec2::new(wheel.position.x, wheel.position.z),
                elevation: wheel.position.y,
                radius: wheel.radius,
                axle: glam::Vec2::new(axle.x, axle.z),
                on_ground: wheel.on_ground,
            }
        })
    }

    /// Punctures tire `wheel` (physics order) on the tire model. Returns whether it was intact.
    pub fn puncture_wheel(&mut self, wheel: usize) -> bool {
        self.vehicle.tire_damage.puncture(wheel)
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
    pub fn water_contacts(
        &self,
        contacts: [Option<Contact>; 4],
    ) -> [Option<crate::scenes::world::vehicle_effects::WaterContact>; 4] {
        std::array::from_fn(|i| {
            let contact = contacts[i]?;
            let wheel = self.vehicle.wheel(i);
            Some(crate::scenes::world::vehicle_effects::WaterContact {
                point: contact.point,
                normal: contact.normal,
                surface: self.vehicle.wheel_surface_tag(i)?,
                slip: wheel.slip,
                slide: wheel.lateral_speed,
                flat: self.vehicle.tire_damage.is_blown(i),
            })
        })
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

    /// The rolling radius of each wheel in the model's wheel order (front left, front right, rear right, rear left).
    pub fn wheel_radii(&self) -> [f32; 4] {
        std::array::from_fn(|i| self.vehicle.wheel(PHYSICS_WHEEL[i]).radius)
    }

    /// Fill `out` with the zones whose glass is cracked, in order. `out` is cleared first and keeps its capacity,
    /// so a caller that reuses it allocates nothing per frame.
    pub fn cracked_zones_into(&self, out: &mut Vec<usize>) {
        out.clear();
        let Some(events) = self.events.as_ref() else { return };
        out.extend((0..events.damage.zone_count()).filter(|&zone| events.damage.zone(zone).glass > 0));
    }

    /// Apply an impact to a zone, as if the car was hit (the `damage` console command). Returns whether it changed.
    pub fn inflict(&mut self, zone: usize, severity: u8, glazed: bool) -> bool {
        let Some(events) = self.events.as_mut() else {
            log::warn!("this car has no damage events; the hit is not applied");
            return false;
        };
        let changed = events.damage.impact(zone, severity, glazed);
        log::info!("damage: zone {zone} severity {severity}, changed {changed}, now {:?}", events.damage.zone(zone));
        changed
    }

    /// Clear the damage; the punctures follow on the next step.
    pub fn repair(&mut self) {
        if let Some(events) = self.events.as_mut() {
            events.damage.repair();
            // The hinged parts of a mod car are on the car again (the ones that fell off stay where they lie).
            events.hidden_slots.retain(|&slot| nfsmw_data::car::mods::hinged::index_of(slot).is_none());
            events.repair_lights();
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

fn next_sound_identity() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    (1 << 63) | NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

mod ai;
mod camera;
mod contact_damage;
mod hud;
mod impact_effects;
mod lights;
mod mod_feed;
mod pair;
mod pair_dispatch;
mod performance;
mod race_launch;
mod sound_contacts;
mod step;
mod terrain;
pub(in crate::scenes::world) use step::StepStart;
pub(in crate::scenes::world) use terrain::Integration;
mod trailer;

#[cfg(test)]
mod tests;
