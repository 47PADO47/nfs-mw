//! What a computer driver needs of the car: its controls set-up, a view of the state and gear changes.

use blackbox_vehicle::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};
use blackbox_vehicle::steering::SteeringDevice;
use blackbox_vehicle::{ControlConfig, Ground};
use glam::{Mat3, Quat, Vec3};

use super::CarSim;
use crate::scenes::world::{road::Spawn, space};

#[cfg(test)]
#[path = "ai_acceleration_tests.rs"]
mod acceleration_tests;

impl CarSim {
    /// Effective input after script, cinematic and arrest overrides; distinct from engine NOS engagement.
    pub fn nos_input(&self) -> bool {
        self.vehicle.controls().nos
    }
    /// Current nitrous capability after tuning; tank level is separate telemetry.
    pub(in crate::scenes::world) fn nitrous_spec(&self) -> blackbox_vehicle::nos::NosSpec {
        let mut spec = self.vehicle.spec().nos;
        let tuning = self.vehicle.tunings.nos.clamp(-1.0, 1.0);
        spec.torque_boost *= 1.0 + 0.25 * tuning;
        spec.nos_capacity *= 1.0 - 0.25 * tuning;
        spec
    }
    /// Original destroyed-on-wheels sleep is independent of retirement brake controls.
    pub fn set_destroyed_non_player(&mut self, destroyed: bool) {
        let delay = (destroyed && !self.collision_role.player).then_some(2.0);
        self.vehicle.set_retirement_sleep_delay(delay);
        let response = (destroyed && !self.collision_role.player && self.collision_role.cop)
            .then(nfsmw_data::car::physics::destroyed_coasting_tires);
        self.vehicle.set_coasting_tires(response);
    }

    pub fn finish_retirement_contacts(&mut self) {
        self.vehicle.finish_retirement_contacts();
    }

    /// Traffic starts supported by the road, including its bank and slope, with tangent velocity.
    /// Spec: `docs/specs/ai-traffic-spawning.md` section 12.
    pub fn place_traffic_moving(&mut self, ground: &dyn Ground, spawn: Spawn, speed: f32) -> bool {
        let [x, _, z] = space::to_physics(spawn.position);
        let top = spawn.position.z + 2.0;
        let Some(hit) = ground.hit(Vec3::new(x, top, z), Vec3::NEG_Y, 100.0) else { return false };
        let Some(up) = hit.normal.try_normalize() else { return false };
        if up.y <= f32::EPSILON || !hit.distance.is_finite() || !speed.is_finite() || !spawn.heading.is_finite() {
            return false;
        }
        let heading = Quat::from_rotation_y(-spawn.heading) * Vec3::Z;
        let forward = Vec3::new(heading.x, -(up.x * heading.x + up.z * heading.z) / up.y, heading.z).normalize();
        let right = up.cross(forward).normalize();
        let orientation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
        let position = Vec3::new(x, top - hit.distance + self.ride_lift() / up.y, z);
        if !position.is_finite() || !orientation.is_finite() {
            return false;
        }
        self.spin = [0.0; 4];
        self.brake_lamp_age = 0.0;
        match speed > 0.0 {
            true => self.vehicle.place_moving(position, orientation, speed),
            false => self.vehicle.place(position, orientation),
        }
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
        true
    }

    /// How far above the road the body's centre stands at rest: the body's half height and the higher axle's ride.
    pub fn ride_lift(&self) -> f32 {
        let spec = self.vehicle.spec();
        let ride = |axle| spec.chassis.axle(axle, self.vehicle.tunings.ride_height).ride;
        spec.dimension.y + ride(0).max(ride(1))
    }

    /// Puts the car's body over the road point `road` (physics space), its nose along `forward` (which may climb or
    /// fall) and level across, rolling at `speed`, with no ground under it: the stand-in of a car off the loaded world
    /// (`ai/car/stand_in.rs`). The next step does not sweep from where the car was.
    pub fn place_kinematic(&mut self, road: Vec3, forward: Vec3, speed: f32) {
        let Some(forward) = forward.try_normalize() else { return };
        let Some(right) = Vec3::Y.cross(forward).try_normalize() else { return };
        let up = forward.cross(right);
        let orientation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
        let position = road + Vec3::Y * self.ride_lift();
        if !position.is_finite() || !orientation.is_finite() || !speed.is_finite() {
            return;
        }
        self.vehicle.place_moving(position, orientation, speed.max(0.0));
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
    }

    /// Makes the car take its controls from an AI: no dead zone, no automatic reverse or idle braking,
    /// the full steering angle for the input, and the gearbox shifts by itself.
    pub fn configure_ai(&mut self) {
        self.collision_role.player = false;
        self.vehicle.set_simple_chassis(Some(nfsmw_data::car::physics::simple_chassis_config()));
        self.vehicle.config = ControlConfig {
            dead_zone: 0.0,
            auto_reverse: false,
            auto_brake: false,
            automatic: true,
            steering_device: SteeringDevice::Ai,
            manual_clutch: false,
            h_shifter: false,
        };
    }

    /// Makes the car take its controls from a person on a computer-driven body (split screen): the player's own control
    /// set-up (dead zone, automatic reverse, idle braking, the pad's steering response) and chassis, so the car drives as the
    /// player's does. `automatic` is the person's gearbox.
    pub fn configure_human(&mut self, automatic: bool) {
        self.collision_role.player = false;
        self.vehicle.set_simple_chassis(None);
        self.vehicle.config = ControlConfig { automatic, ..ControlConfig::default() };
    }

    /// Ordinary traffic selects its own suspension, independently of police and racer controls.
    pub fn configure_traffic(&mut self) {
        self.configure_ai();
        let config = nfsmw_data::car::physics::traffic_chassis_config(self.vehicle.spec());
        self.vehicle.set_rolling_chassis(Some(config));
    }

    /// What an AI driver needs to know of the car.
    pub fn driver_view(&self) -> blackbox_driver::VehicleView {
        let v = &self.vehicle;
        let velocity = v.linear_velocity();
        blackbox_driver::VehicleView {
            position: v.position(),
            forward: v.rotation().z_axis,
            forward_speed: v.forward_speed(),
            planar_speed: Vec3::new(velocity.x, 0.0, velocity.z).length(),
            gear_is_reverse: v.gear() == GEAR_REVERSE,
            max_steer: v.maximum_steering_degrees().to_radians(),
            in_shock: false,
            staging: false,
        }
    }

    /// Terrain-cache supporting normal, in physics space; upright fallback without a cached face.
    pub fn ground_normal(&self) -> Vec3 {
        self.vehicle.ground_reference.map_or(Vec3::Y, |ground| ground.normal)
    }

    /// The body's velocity in physics space.
    pub fn velocity(&self) -> Vec3 {
        self.vehicle.linear_velocity()
    }

    pub fn shift_reverse(&mut self) {
        self.vehicle.shift_to(GEAR_REVERSE);
    }

    pub fn shift_first(&mut self) {
        self.vehicle.shift_to(GEAR_FIRST);
    }

    pub fn shift_neutral(&mut self) {
        self.vehicle.shift_to(GEAR_NEUTRAL);
    }
}
