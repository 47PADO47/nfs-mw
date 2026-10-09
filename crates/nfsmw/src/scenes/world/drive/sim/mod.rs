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
use nfsmw_data::car::physics::{CarPhysics, SurfaceTable, WallSpec};

use super::input::DriveInput;
use super::rig::CarPose;
use super::walls::{self, Impact};
use crate::scenes::world::effects::{self, Contact};
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
        Self {
            vehicle: Vehicle::new(physics.spec),
            pivot: physics.bounds.pivot,
            rest_height,
            spin: [0.0; 4],
            walls: physics.walls,
            visual_tires: None,
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

    /// Put the car on the road at `spawn`, standing still. False when `ground` has nothing there.
    pub fn place(&mut self, ground: &dyn Ground, spawn: Spawn) -> bool {
        self.place_moving(ground, spawn, 0.0)
    }

    /// Put the car on the road at `spawn`, already rolling forward at `speed` m/s.
    pub fn place_moving(&mut self, ground: &dyn Ground, spawn: Spawn, speed: f32) -> bool {
        self.spin = [0.0; 4];
        let [x, _, z] = space::to_physics(spawn.position);
        // The heading is counter-clockwise from +X in the world; the library's yaw is about the up axis
        // from +z, which is the other way round.
        let top = spawn.position.z + 2.0;
        self.vehicle.place_on_ground_moving(ground, x, z, top, -spawn.heading, speed)
    }

    /// Whether the gearbox shifts by itself (the transmission setting).
    pub fn set_automatic(&mut self, automatic: bool) {
        self.vehicle.config.automatic = automatic;
    }

    /// One physics step. `world` is the collision the body's walls are tested against (the ground is
    /// `ground`); without it the car only meets the road.
    pub fn step(
        &mut self,
        input: &DriveInput,
        ground: &dyn Ground,
        world: Option<(&CollisionWorld, &PropWorld)>,
    ) -> Impact {
        let input = InputState {
            throttle: input.throttle,
            brake: input.brake,
            steer: input.steer,
            handbrake: f32::from(u8::from(input.handbrake)),
            nos: input.nos,
            shift_up: input.shift_up,
            shift_down: input.shift_down,
        };
        self.vehicle.step(FIXED_STEP, &input, ground);
        let impact = match world {
            Some((collision, props)) => walls::resolve(
                &mut self.vehicle,
                &walls::world_cast(collision),
                &walls::world_props(props),
                &self.walls,
            ),
            None => Impact::default(),
        };
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

    /// Whether the state is usable (no NaN or runaway values).
    pub fn is_finite(&self) -> bool {
        let v = &self.vehicle;
        v.position().is_finite() && v.orientation().is_finite() && v.linear_velocity().is_finite()
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
mod pair;
mod trailer;

#[cfg(test)]
mod tests;
