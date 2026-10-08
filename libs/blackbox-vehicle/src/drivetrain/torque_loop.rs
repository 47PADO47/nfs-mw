//! One tick of the engine torque loop (spec section 3): engine speed against transmission speed, the
//! clutch coupling, launch behaviour, limits, and the torque handed to the chassis.

use super::powertrain::Powertrain;
use super::spec::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};
use super::wheels::{driven_speeds, write_back};
use crate::engine::ClutchState;
use crate::math::{graph, rad_to_rpm, ramp};

const CLUTCH_STIFFNESS: f32 = 20.0;
const CLUTCH_LIMITER: f32 = 300.0;
/// Clutch play table: `(1000 * net torque, factor)`.
const CLUTCH_PLAY: [(f32, f32); 5] = [(-10.0, 1.0), (-7.5, 0.96), (-3.5, 0.925), (-0.3, 0.875), (-0.05, 0.0)];

pub(super) struct LoopCtx<'a> {
    pub dt: f32,
    pub av: &'a mut [f32; 4],
    pub grounded: [bool; 4],
    pub drive_slip: f32,
    pub speed: f32,
}

impl Powertrain {
    /// Runs the torque loop and returns the drive torque at the wheels (N m).
    pub(super) fn torque_loop(&mut self, c: &mut LoopCtx<'_>) -> f32 {
        let dt = c.dt;
        let min_w = self.engine.idle_rad();
        let max_w = self.engine.red_line_rad();
        let reverse = self.gear == GEAR_REVERSE;
        let neutral = self.gear == GEAR_NEUTRAL;
        let dir = if reverse { -1.0 } else { 1.0 };
        let driven = (self.trans.front_driven(), self.trans.rear_driven());
        let mut ratio = self.trans.ratio(self.gear) * self.trans.final_gear * dir;
        let rpm = rad_to_rpm(self.omega);

        if self.trans.torque_converter > 0.0 {
            let mut conv =
                self.trans.torque_converter * self.throttle * (1.0 - ramp(rpm, self.engine.idle, self.peak_torque_rpm));
            if self.shifting() {
                conv *= self.clutch.factor();
            }
            ratio *= 1.0 + conv;
        }
        if ratio == 0.0 && !neutral {
            return 0.0;
        }

        let engine_t = self.engine.torque_nm(rpm) * (1.0 + self.induction.boost) * self.nos.boost;
        let mut braking_t = self.engine.braking_torque(engine_t, rpm);

        let (free, locked) = driven_speeds(c.av, driven, reverse);
        let omega_trans_old = self.omega_trans;
        self.omega_trans = min_w + locked * ratio * (max_w - min_w) / max_w;
        let trans_accel = (self.omega_trans - omega_trans_old) / dt;

        // Wheels driving the engine faster than it drives them: braking must still oppose the motion.
        if !neutral && self.clutch.is_engaged() && braking_t * free * dir > 0.0 {
            braking_t = -braking_t;
        }
        let throttle = self.throttle;
        let total_t = engine_t * throttle + braking_t * (1.0 - throttle);
        self.engine_braking = total_t < 0.0;
        let on_ground = c.grounded.iter().filter(|g| **g).count();
        let wheels_ratio = (on_ground as f32 / 4.0).max(0.25);
        let inertia = self.engine.inertia(neutral);
        let first_or_reverse = !neutral && self.gear <= GEAR_FIRST;

        let (mut drive_t, mut road_t) = (0.0_f32, 0.0_f32);
        let diff_rpm = rad_to_rpm(self.omega - self.omega_trans);
        if !neutral {
            match self.clutch.state {
                ClutchState::Engaged => {
                    drive_t = total_t;
                    road_t = -total_t * wheels_ratio;
                    // Make the engine and the wheels agree on acceleration.
                    let ae = (total_t + road_t) / inertia;
                    let diff = ae - trans_accel;
                    let r2 = ratio * ratio;
                    let mut response = 1.0 / inertia;
                    if driven.0 {
                        response += self.trans.torque_split * 0.1 * r2 * 0.5;
                    }
                    if driven.1 {
                        response += (1.0 - self.trans.torque_split) * 0.1 * r2 * 0.5;
                    }
                    let r = diff / response;
                    drive_t += r.min(0.0);
                    road_t -= r * wheels_ratio;
                }
                ClutchState::Engaging => {
                    let d = (self.omega - self.omega_trans).clamp(-CLUTCH_LIMITER, CLUTCH_LIMITER);
                    let mut stiffness = CLUTCH_STIFFNESS;
                    if self.prev_rpm_diff * diff_rpm < 0.0
                        && self.prev_rpm_diff.abs() > 2000.0
                        && diff_rpm.abs() > 2000.0
                    {
                        stiffness *= 0.5;
                    }
                    let clutch_t = d * stiffness * self.clutch.factor();
                    drive_t += clutch_t;
                    road_t -= clutch_t * wheels_ratio;
                }
                ClutchState::Disengaged => {}
            }
        }
        self.prev_rpm_diff = diff_rpm;

        if !neutral && self.clutch.is_engaged() && on_ground > 0 {
            if throttle > 0.2 && c.drive_slip > 0.1 && self.gear <= GEAR_FIRST && self.omega_trans < self.omega {
                // Launch: lock the engine to the wheels and let the wheels follow it.
                self.omega_trans = self.omega;
                write_back(c.av, c.grounded, driven, reverse, self.omega / ratio, c.speed);
            } else {
                let lim = total_t.abs();
                road_t += ((self.omega_trans - self.omega) * CLUTCH_STIFFNESS).clamp(-lim, lim);
            }
        }

        if first_or_reverse && self.clutch.is_engaged() {
            let net = total_t + road_t;
            if road_t < total_t && net < 0.0 {
                let play = graph(&CLUTCH_PLAY, 1000.0 * net);
                road_t += play * net * self.trans.clutch_slip;
            }
            if throttle > 0.0 && road_t * total_t < 0.0 {
                let slip = self.trans.clutch_slip
                    * throttle
                    * (1.0 - throttle * ramp(rpm, self.engine.idle, self.peak_torque_rpm));
                road_t *= (1.0 - slip) * (1.0 - slip);
            }
        }

        let alpha = (total_t + road_t) / inertia;
        self.omega = (self.omega + alpha * dt).clamp(min_w, max_w);

        if ratio != 0.0 {
            let limit = max_w / ratio.abs();
            for (i, av) in c.av.iter_mut().enumerate() {
                let is_driven = if i < 2 { driven.0 } else { driven.1 };
                if is_driven && !c.grounded[i] {
                    *av = if *av * dir < 0.0 { 0.0 } else { av.clamp(-limit, limit) };
                }
            }
            if self.omega_trans > max_w {
                if drive_t * ratio > 0.0 {
                    drive_t = 0.0;
                }
                self.omega_trans = max_w;
                write_back(c.av, c.grounded, driven, reverse, max_w / ratio, c.speed);
            }
        }
        drive_t * ratio * self.trans.efficiency(self.gear)
    }
}
