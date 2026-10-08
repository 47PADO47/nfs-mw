//! The car's physics (`blackbox-vehicle`) seen from the game: input in, a pose to draw out, and the
//! change of axes between the library's physics space and the render space.
//!
//! Physics space is x right, y up, z forward (left-handed); the world and the car models are x forward,
//! y left, z up. The library numbers the wheels 0 front left, 1 front right, 2 rear left, 3 rear right;
//! the car model has 2 rear right, 3 rear left.

use blackbox_collision::CollisionWorld;
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
    /// The engine's red line, rpm: the end of the tachometer scale.
    pub red_line: f32,
    /// The engine's idle speed, rpm.
    pub idle: f32,
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

    pub(super) fn set_visual_tires(&mut self, tires: Option<[super::visual_tires::VisualTire; 4]>) {
        self.visual_tires = tires;
    }

    /// Put the car on the road at `spawn`, standing still. False when `ground` has nothing there.
    pub fn place(&mut self, ground: &dyn Ground, spawn: Spawn) -> bool {
        self.spin = [0.0; 4];
        let [x, _, z] = space::to_physics(spawn.position);
        // The heading is counter-clockwise from +X in the world; the library's yaw is about the up axis
        // from +z, which is the other way round.
        let top = spawn.position.z + 2.0;
        if self.vehicle.place_on_ground(ground, x, z, top, -spawn.heading) {
            return true;
        }
        false
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
            idle: v.powertrain().engine_spec().idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use blackbox_vehicle::ground::NoGround;
    use blackbox_vehicle::{FlatGround, VehicleSpec};
    use nfsmw_data::car::physics::CarBounds;

    use super::*;

    fn sim() -> CarSim {
        let spec = VehicleSpec::example();
        let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::new(0.0, spec.dimension.y, 0.1) };
        CarSim::new(CarPhysics { spec, bounds, walls: WallSpec::default() }, [0.2; 4])
    }

    fn spawn_at(heading: f32) -> Spawn {
        Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading }
    }

    fn drive(sim: &mut CarSim, ground: &dyn Ground, input: DriveInput, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            let _ = sim.step(&input, ground, None);
        }
    }

    #[test]
    fn placing_puts_the_car_on_the_road_facing_the_heading() {
        // The road at render z = 5 is physics y = 5.
        let ground = FlatGround::new(5.0);
        let mut sim = sim();
        assert!(sim.place(&ground, spawn_at(0.0)));
        let pose = sim.pose();
        assert!(
            (pose.position.x - 100.0 + 0.1).abs() < 0.5 && (pose.position.y - 50.0).abs() < 0.5,
            "{:?}",
            pose.position
        );
        assert!(pose.position.z > 5.0 && pose.position.z < 6.0, "{:?}", pose.position);
        // Facing +X in the world: the model's forward axis is the world's x axis.
        let forward = pose.rotation * Vec3::X;
        assert!((forward - Vec3::X).length() < 1e-4, "{forward:?}");
        assert!(((pose.rotation * Vec3::Z) - Vec3::Z).length() < 1e-4, "up is up");

        // Headed along +Y (a quarter turn to the left), the car points along +Y.
        assert!(sim.place(&ground, spawn_at(std::f32::consts::FRAC_PI_2)));
        let forward = sim.pose().rotation * Vec3::X;
        assert!((forward - Vec3::Y).length() < 1e-4, "{forward:?}");
    }

    #[test]
    fn full_throttle_accelerates_forward_and_up_through_the_gears() {
        let ground = FlatGround::new(5.0);
        let mut sim = sim();
        assert!(sim.place(&ground, spawn_at(0.0)));
        let start = sim.pose().position;
        drive(&mut sim, &ground, DriveInput { throttle: 1.0, ..DriveInput::default() }, 8.0);
        let t = sim.telemetry();
        let moved = sim.pose().position - start;
        assert!(t.speed_mps > 20.0, "speed {} m/s", t.speed_mps);
        assert!(moved.x > 50.0 && moved.y.abs() < 2.0 && moved.z.abs() < 0.5, "moved {moved:?}");
        assert!(t.gear >= 2, "gear {}", t.gear);
        assert!(sim.is_finite());
        assert_eq!(t.wheels_on_ground, 4);
        // The wheels have rolled forward: the spin grew.
        assert!(sim.pose().wheels.iter().all(|w| w.spin > 10.0), "{:?}", sim.pose().wheels);
    }

    #[test]
    fn steering_right_turns_the_car_to_the_right() {
        let ground = FlatGround::new(5.0);
        let mut sim = sim();
        assert!(sim.place(&ground, spawn_at(0.0)));
        drive(&mut sim, &ground, DriveInput { throttle: 0.6, ..DriveInput::default() }, 3.0);
        drive(&mut sim, &ground, DriveInput { throttle: 0.5, steer: 1.0, ..DriveInput::default() }, 2.0);
        let pose = sim.pose();
        // Right of +X is -Y.
        assert!(pose.position.y < 50.0 - 1.0, "went {:?}", pose.position);
        let forward = pose.rotation * Vec3::X;
        assert!(forward.y < -0.1, "{forward:?}");
        // The front wheels show a right turn as a negative (left-positive) angle; the rear wheels none.
        assert!(pose.wheels[0].steer < 0.0 && pose.wheels[1].steer < 0.0);
        assert!(pose.wheels[2].steer == 0.0 && pose.wheels[3].steer == 0.0);
    }

    #[test]
    fn braking_stops_the_car_and_reverse_follows() {
        let ground = FlatGround::new(5.0);
        let mut sim = sim();
        assert!(sim.place(&ground, spawn_at(0.0)));
        drive(&mut sim, &ground, DriveInput { throttle: 1.0, ..DriveInput::default() }, 4.0);
        drive(&mut sim, &ground, DriveInput { brake: 1.0, ..DriveInput::default() }, 8.0);
        assert!(sim.telemetry().speed_mps < -0.5, "the held brake pedal reverses: {}", sim.telemetry().speed_mps);
        assert_eq!(sim.telemetry().gear, -1);
    }

    #[test]
    fn nothing_below_means_no_car() {
        assert!(!sim().place(&NoGround, spawn_at(0.0)));
    }

    #[test]
    fn the_model_follows_the_body_not_the_other_way_round() {
        // Wheel travel is small on flat ground at rest and the car does not sink through the road.
        let ground = FlatGround::new(5.0);
        let mut sim = sim();
        assert!(sim.place(&ground, spawn_at(0.0)));
        drive(&mut sim, &ground, DriveInput::default(), 3.0);
        let pose = sim.pose();
        assert!(pose.position.z > 5.0 && pose.position.z < 5.6, "height {}", pose.position.z);
        assert!(pose.wheels.iter().all(|w| w.travel.abs() < MAX_VISIBLE_TRAVEL), "{:?}", pose.wheels);
    }
}
