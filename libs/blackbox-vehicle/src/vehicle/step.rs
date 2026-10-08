//! One fixed step of the whole vehicle, in the order of the spec: integrate last step's forces, shape the
//! controls, steer, run the engine, then aero, the wheels, damping and body contacts.

use glam::Vec3;

use super::Vehicle;
use crate::aero::{self, AeroInput};
use crate::drivetrain::{GEAR_FIRST, GEAR_REVERSE, TickInput, split_drive_torque};
use crate::ground::Ground;
use crate::input::{GearRequest, InputContext, InputState, shape};
use crate::math::clamp_signed;
use crate::rigid_body::BodyState;
use crate::steering::{SteeringInput, ackermann};
use crate::suspension::center_of_gravity;

impl Vehicle {
    /// Advances the car by `dt` seconds (use [`crate::FIXED_STEP`]; the tuning assumes 60 Hz).
    ///
    /// Forces computed in this step move the body in the next one, so the position you read afterwards
    /// lags the forces by one step.
    pub fn step(&mut self, dt: f32, input: &InputState, ground: &dyn Ground) {
        if dt.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !dt.is_finite() {
            return;
        }
        self.body.begin_frame(dt);
        self.time += dt;
        match self.body.state {
            BodyState::Frozen => return,
            BodyState::Asleep => {
                if input.throttle > 0.0 || input.handbrake > 0.0 {
                    self.body.wake();
                } else {
                    return;
                }
            }
            BodyState::Awake => {}
        }
        let collided = std::mem::take(&mut self.collided);
        self.wheels_on_ground_prev = self.wheels_on_ground;

        let rot = self.body.rotation();
        let local_velocity = rot.transpose() * self.body.linear_velocity;
        let speed = self.body.linear_velocity.length();
        self.body_slip = if local_velocity.z < 1.0 { 0.0 } else { local_velocity.x.atan2(local_velocity.z) };

        self.update_controls(input, local_velocity.z);
        self.body.set_cog(center_of_gravity(
            &self.spec.chassis,
            self.spec.dimension.y,
            self.tunings.ride_height,
            self.wheels_on_ground_prev > 0,
        ));
        self.update_steering(dt, local_velocity.z);

        self.begin_tires(speed);
        self.run_powertrain(dt, speed);
        self.tune_tires(dt, speed);
        self.apply_aero();

        let out = self.wheel_forces(dt, ground);
        if self.wheels_on_ground > 0 {
            let up = self.body.rotation().y_axis;
            let mut torque = out.torque;
            // The yaw component of the tire torque is scaled by YAW_SPEED.
            let yaw = up.dot(torque);
            torque += up * (yaw * self.spec.tires.yaw_speed - yaw);
            self.body.apply_force(out.force);
            self.body.apply_torque(torque);
            if out.lift_need > 0.0 {
                self.body.position.y += out.lift_need;
            }
        }
        self.damp_landing();
        self.damp_creep(collided);

        self.body.apply_drag();
        let corner_contacts = self.collide_body_with_ground(ground);
        if self.wheels_on_ground == 0 {
            self.body.update_sleep(corner_contacts);
        }
        self.sanitize();
    }

    fn update_controls(&mut self, input: &InputState, forward_speed: f32) {
        let ctx = InputContext { forward_speed, gear: self.powertrain.gear(), disabled: self.disabled };
        let controls = shape(input, &self.config, &ctx);
        match controls.gear_request {
            Some(GearRequest::Reverse) => {
                self.powertrain.shift(GEAR_REVERSE);
            }
            Some(GearRequest::First) => {
                self.powertrain.shift(GEAR_FIRST);
            }
            Some(GearRequest::Shift(dir)) => self.powertrain.request_shift(dir, self.config.automatic),
            None => {}
        }
        self.controls = controls;
    }

    fn update_steering(&mut self, dt: f32, forward_speed: f32) {
        let c = self.controls;
        let angle = self.steering.update(&SteeringInput {
            dt,
            steer: c.steering,
            gas: c.gas,
            brake: c.brake,
            ebrake: c.handbrake,
            forward_speed,
            tire_steering: self.spec.tires.steering,
            tuning: self.tunings.steering,
            device: self.config.steering_device,
            rear_slip_deg: [self.tires[2].slip_angle * 360.0, self.tires[3].slip_angle * 360.0],
        });
        self.wheel_angles = ackermann(angle, self.spec.chassis.wheel_base, self.spec.chassis.track_width[0]);
    }

    fn run_powertrain(&mut self, dt: f32, speed: f32) {
        let trans = &self.spec.transmission;
        let (front, rear) = (trans.front_driven(), trans.rear_driven());
        let dir = if self.powertrain.gear() == GEAR_REVERSE { -1.0 } else { 1.0 };
        let driven: Vec<usize> = (0..4).filter(|i| if *i < 2 { front } else { rear }).collect();
        let drive_slip = if driven.is_empty() {
            0.0
        } else {
            driven.iter().map(|&i| self.tires[i].slip).sum::<f32>() / driven.len() as f32 * dir
        };
        let max_wheel_slip = self.tires.iter().map(|t| t.slip.abs()).fold(0.0, f32::max);
        let mut av: [f32; 4] = std::array::from_fn(|i| self.tires[i].av);
        let grounded = self.loaded;
        let drive_torque = self.powertrain.tick(&mut TickInput {
            dt,
            gas: self.controls.gas,
            nos_held: self.controls.nos,
            blown: self.disabled,
            automatic: self.config.automatic,
            speed,
            wheel_av: &mut av,
            grounded,
            drive_slip,
            max_wheel_slip,
            induction_tuning: self.tunings.induction,
            nos_tuning: self.tunings.nos,
        });
        for (t, a) in self.tires.iter_mut().zip(av) {
            t.av = a;
        }
        let torques = split_drive_torque(drive_torque, &self.spec.transmission, av, grounded);
        for (t, q) in self.tires.iter_mut().zip(torques) {
            t.drive_torque = q;
        }
    }

    fn apply_aero(&mut self) {
        let rot = self.body.rotation();
        let f = aero::forces(
            &self.spec.aero,
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
        if f.drag != Vec3::ZERO {
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
