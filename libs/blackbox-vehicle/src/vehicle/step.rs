//! Shared body and vehicle-response phases. Spec: `shared-vehicle-phases.md`.

use glam::Vec3;

use super::Vehicle;
use crate::aero::{self, AeroInput};
use crate::drivetrain::{GEAR_FIRST, GEAR_REVERSE, TickInput};
use crate::ground::Ground;
use crate::input::{ControlConfig, GearRequest, InputContext, InputState, shape};
use crate::math::clamp_signed;
use crate::rigid_body::BodyState;
use crate::steering::{SteeringDevice, SteeringInput, ackermann};

/// Rear slip angle (turns, 1.0 = 360 deg) past which the car counts as drifting. About 14 degrees.
const DRIFT_SLIP: f32 = 0.04;

impl Vehicle {
    /// Advances the car by `dt` seconds (use [`crate::FIXED_STEP`]; the tuning assumes 60 Hz).
    ///
    /// Forces computed in this step move the body in the next one, so the position you read afterwards
    /// lags the forces by one step.
    pub fn step(&mut self, dt: f32, input: &InputState, ground: &dyn Ground) {
        self.begin_body_step(dt);
        self.finish_body_step(dt, ground);
        self.respond_step(dt, input, ground);
    }

    /// Integrates previous forces. The host resolves all body pairs after this phase.
    pub fn begin_body_step(&mut self, dt: f32) {
        if dt.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !dt.is_finite() {
            return;
        }
        self.tire_damage.advance(dt);
        self.begin_retirement_frame();
        self.body_contacts.clear();
        let previous_position = self.body.position;
        self.body.begin_frame(dt);
        self.hold_staged_body(previous_position);
        self.time += dt;
    }

    /// Finishes body collision and drag before any engine or suspension response.
    pub fn finish_body_step(&mut self, dt: f32, ground: &dyn Ground) {
        if !dt.is_finite() || dt <= 0.0 || self.body.state != BodyState::Awake {
            return;
        }
        let corner_contacts = self.collide_body_with_ground(ground, dt);
        self.body_ground_contact = corner_contacts > 0;
        self.body.apply_drag();
        if self.tire_integration == crate::tires::TireIntegration::Reference {
            if let Some(contacts) = self.reference_sleep_contacts(corner_contacts) {
                self.body.update_sleep(contacts);
            }
            return;
        }
        if self.wheels_on_ground == 0 && !self.staging() && !self.collided {
            self.body.update_sleep(corner_contacts);
        }
    }

    /// Computes this tick's driving response from the post-contact body state.
    pub fn respond_step(&mut self, dt: f32, input: &InputState, ground: &dyn Ground) {
        if !dt.is_finite() || dt <= 0.0 || self.body.state == BodyState::Frozen {
            return;
        }
        let rot = self.body.rotation();
        let local_velocity = rot.transpose() * self.body.linear_velocity;
        // Input and brake-lamp consumers remain current even when chassis response sleeps.
        self.update_controls(input, local_velocity.z);
        let reference = self.tire_integration == crate::tires::TireIntegration::Reference;
        if reference {
            self.advance_retirement_support(dt);
        }
        match self.body.state {
            BodyState::Frozen => return,
            BodyState::Asleep if reference => {
                if self.retired() {
                    self.update_launch(dt, self.body.linear_velocity.length());
                    self.run_powertrain(dt, self.body.linear_velocity.length());
                    self.sanitize();
                    return;
                }
            }
            BodyState::Asleep => {
                if self.staging() || input.throttle > 0.0 || input.handbrake > 0.0 {
                    self.body.wake();
                } else {
                    return;
                }
            }
            BodyState::Awake => {}
        }
        let collided = std::mem::take(&mut self.collided);
        self.wheels_on_ground_prev = self.wheels_on_ground;

        let speed = self.body.linear_velocity.length();
        let angular_velocity = self.body.angular_velocity;
        self.body_slip = if local_velocity.z < 1.0 { 0.0 } else { local_velocity.x.atan2(local_velocity.z) };

        self.update_chassis_cog();
        self.update_steering(dt, local_velocity.z);
        self.apply_wall_steering();

        self.update_launch(dt, speed);
        self.begin_tires(speed);
        let drive_torque = self.run_powertrain(dt, speed);
        self.distribute_drive(drive_torque, self.tires.map(|t| t.av), self.loaded);
        self.tune_tires(dt, speed, angular_velocity);
        self.apply_aero();

        let out = self.wheel_forces(dt, ground, angular_velocity);
        if !reference {
            self.advance_retirement_support(dt);
        }
        if self.wheels_on_ground > 0 {
            let up = self.body.rotation().y_axis;
            let mut torque = out.torque;
            // The yaw component of the tire torque is scaled by YAW_SPEED.
            let yaw = up.dot(torque);
            torque += up * (yaw * self.spec.tires.yaw_speed - yaw);
            self.body.apply_force(out.force);
            self.body.apply_torque(torque);
        }
        if out.lift_need > 0.0 {
            self.body.position.y += out.lift_need;
            for patch in &mut self.patch_positions {
                patch.y += out.lift_need;
            }
        }
        let angular_velocity = self.damp_landing(angular_velocity);
        self.apply_jump(dt, speed, angular_velocity);
        self.damp_creep(collided, angular_velocity);

        self.sanitize();
    }

    fn update_controls(&mut self, input: &InputState, forward_speed: f32) {
        // The player's shift action precedes auto-reverse. Shape from the gear
        // actually accepted by the gearbox, including reverse exit this tick.
        let mut pedals = *input;
        // A gearbox held in neutral (an engine that is off) takes no gear request and no automatic reverse.
        let held = self.powertrain.neutral_held();
        if held {
            pedals.gear_select = None;
        }
        if !self.disabled && !held && (self.config.automatic || input.gear_select.is_none()) {
            let dir = i32::from(input.shift_up) - i32::from(input.shift_down);
            let desired =
                (self.powertrain.gear() as i64 + i64::from(dir)).clamp(1, self.powertrain.top_gear() as i64) as usize;
            if dir != 0 && (!self.staging() || desired <= GEAR_FIRST) {
                self.powertrain.request_shift(dir, self.config.automatic);
            }
        }
        pedals.shift_up = false;
        pedals.shift_down = false;
        let ctx = InputContext { forward_speed, gear: self.powertrain.gear(), disabled: self.disabled };
        let config = ControlConfig { auto_reverse: self.config.auto_reverse && !held, ..self.config };
        let controls = shape(&pedals, &config, &ctx);
        match controls.gear_request {
            Some(GearRequest::Reverse) => {
                self.powertrain.shift(GEAR_REVERSE);
            }
            Some(GearRequest::First) => {
                self.powertrain.shift(GEAR_FIRST);
            }
            Some(GearRequest::Shift(dir)) => self.powertrain.request_shift(dir, self.config.automatic),
            Some(GearRequest::Select(gear)) => {
                self.powertrain.select_gear(gear, forward_speed);
            }
            None => {}
        }
        self.controls = controls;
    }

    pub(super) fn update_steering(&mut self, dt: f32, forward_speed: f32) {
        if self.rolling_steering() {
            return;
        }
        let c = self.controls;
        let tire_steering = self.simple_steering_scale();
        let normal_angle = self.steering.update(&SteeringInput {
            dt,
            steer: c.steering,
            gas: c.gas,
            brake: c.brake,
            ebrake: c.handbrake,
            forward_speed,
            tire_steering,
            tuning: self.tunings.steering,
            device: match self.simple_chassis.is_some() {
                true => SteeringDevice::Ai,
                false => self.config.steering_device,
            },
            rear_slip_deg: [self.tires[2].slip_angle * 360.0, self.tires[3].slip_angle * 360.0],
        });
        let angle = match self.maneuver.steering_degrees {
            Some(degrees) => c.steering * degrees,
            None => normal_angle,
        };
        self.wheel_angles = self
            .forced_wheel_angles
            .unwrap_or_else(|| ackermann(angle, self.spec.chassis.wheel_base, self.spec.chassis.track_width[0]));
    }

    fn run_powertrain(&mut self, dt: f32, speed: f32) -> f32 {
        let trans = &self.spec.transmission;
        let (front, rear) = match self.rolling_chassis {
            Some(config) => (config.front_drive_share > 0.0, config.front_drive_share < 1.0),
            None => (trans.front_driven(), trans.rear_driven()),
        };
        let dir = if self.powertrain.gear() == GEAR_REVERSE { -1.0 } else { 1.0 };
        let is_driven = |i: usize| if i < 2 { front } else { rear };
        let count = (0..4).filter(|&i| is_driven(i)).count();
        let drive_slip = if count == 0 {
            0.0
        } else {
            (0..4).filter(|&i| is_driven(i)).map(|i| self.tires[i].slip).sum::<f32>() / count as f32 * dir
        };
        let max_wheel_slip = self.tires.iter().map(|t| t.slip).fold(0.0, f32::max);
        let reference = self.tire_integration == crate::tires::TireIntegration::Reference;
        let mut av: [f32; 4] = std::array::from_fn(|i| match reference {
            true => self.tires[i].display_av(self.params[i].radius, self.corners[i].compression > 0.0),
            false => self.tires[i].av,
        });
        let grounded = self.loaded;
        let wheel_contacts = match reference {
            true => self.corners.map(|corner| corner.compression > 0.0),
            false => grounded,
        };
        let drive_torque = self.powertrain.tick_with_loaded_wheels(
            &mut TickInput {
                dt,
                gas: self.controls.gas,
                nos_held: self.controls.nos,
                can_recharge_nos: self.can_recharge_nos,
                blown: self.disabled,
                automatic: self.config.automatic,
                speed,
                wheel_av: &mut av,
                grounded: wheel_contacts,
                drifting: self.is_drifting(),
                drive_slip,
                max_wheel_slip,
                induction_tuning: self.tunings.induction,
                nos_tuning: self.tunings.nos,
                clutch_pedal: self.controls.clutch,
            },
            self.wheels_on_ground,
        );
        let writes = self.powertrain.wheel_writes();
        for (i, t) in self.tires.iter_mut().enumerate() {
            if self.rolling_chassis.is_none() && (!reference || writes[i]) {
                t.av = av[i];
            }
        }
        drive_torque
    }

    /// A drift: a grounded rear tire slides sideways past [`DRIFT_SLIP`] (turns, 1.0 = 360 deg).
    fn is_drifting(&self) -> bool {
        (2..4).any(|i| self.loaded[i] && self.tires[i].slip_angle.abs() > DRIFT_SLIP)
    }

    pub(super) fn apply_aero(&mut self) {
        if self.rolling_chassis.is_some() {
            let velocity = self.body.linear_velocity;
            self.body.apply_force(-velocity * velocity.length() * self.spec.aero.drag_coefficient);
            return;
        }
        let rot = self.body.rotation();
        if self.maneuver.extra_downward_acceleration != 0.0 {
            self.body.apply_force(-rot.y_axis * self.spec.mass * self.maneuver.extra_downward_acceleration);
        }
        let aero = crate::aero::AeroSpec {
            drag_coefficient: self.spec.aero.drag_coefficient * self.maneuver.drag_scale,
            ..self.spec.aero
        };
        let aero_forces = match self.tire_integration {
            crate::tires::TireIntegration::Reference => aero::reference_forces,
            crate::tires::TireIntegration::Regularized => aero::forces,
        };
        let f = aero_forces(
            &aero,
            &AeroInput {
                linear_velocity: self.body.linear_velocity,
                rotation: rot,
                gas: self.controls.gas,
                ground_effect: 0.25 * self.wheels_on_ground_prev as f32,
                any_wheel_on_ground: self.wheels_on_ground_prev > 0,
                tuning: self.tunings.aerodynamics,
            },
            self.geometry.front_z(),
            self.geometry.rear_z(),
        );
        let reference_drag =
            self.tire_integration == crate::tires::TireIntegration::Reference && self.maneuver.drag_scale > 0.0;
        if reference_drag || f.drag != Vec3::ZERO {
            let at = self.body.world_cog() - rot.y_axis * f.drag_drop;
            self.body.apply_force_at(f.drag, at);
        }
        if f.downforce != Vec3::ZERO {
            let at = match f.downforce_z {
                Some(z) => self.body.position + rot * Vec3::new(0.0, self.body.cog().y, z),
                None => self.body.world_cog(),
            };
            self.body.apply_force_at(f.downforce, at);
        }
    }

    /// Keeps NaN and infinity out of the state, whatever the input was.
    fn sanitize(&mut self) {
        let b = &mut self.body;
        if !(b.position.is_finite()
            && b.linear_velocity.is_finite()
            && b.angular_velocity.is_finite()
            && b.orientation.is_finite())
        {
            // Nothing sensible is left: put the body upright where it was, at rest.
            let position = if b.position.is_finite() { b.position } else { Vec3::ZERO };
            let yaw_only = if b.orientation.is_finite() { b.orientation } else { glam::Quat::IDENTITY };
            b.place(position, yaw_only);
        }
        for t in &mut self.tires {
            t.av = clamp_signed(t.av, 5000.0);
        }
    }
}
