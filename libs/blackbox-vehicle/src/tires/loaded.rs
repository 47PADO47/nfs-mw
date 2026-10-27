//! The loaded-tire update (spec section 3.4) and the unloaded one (section 3.6).

use std::f32::consts::TAU;

use super::lateral::lateral_force;
use super::state::{LoadedInput, ROLLING_FRICTION, Tire, TireParams, WHEEL_INERTIA};
use crate::math::{lerp, ms_to_mph};

const EPS: f32 = 1e-6;
/// Below this skid speed (m/s) the sliding friction ramps up linearly instead of jumping to its full
/// value in the direction of a tiny slip, which made the force flip sign from step to step.
const SKID_FLOOR: f32 = 0.5;
/// The friction ellipse is this much longer along the driven axis while power is applied.
const ELLIPSE_RATIO: f32 = 1.5;
/// Static to dynamic ratio of the brake lock test.
const LOCK_RATIO: f32 = 1.2;
/// Brake lock test: N m per rad/s of wheel speed.
const LOCK_AV_FACTOR: f32 = 100.0;

fn gripping_feedback(angular_acc: f32, radius: f32, forward_acc: f32, reference: bool) -> f32 {
    let delta = angular_acc * radius - forward_acc;
    match reference {
        true => (delta / radius) * WHEEL_INERTIA,
        false => (delta * WHEEL_INERTIA) / radius,
    }
}

fn scale_traction(traction: f32, slip: f32, limit: f32, reference: bool) -> f32 {
    match reference {
        true => (traction / slip.abs()) * limit,
        false => traction * (limit / slip.abs()),
    }
}

impl Tire {
    /// Rear wheels grip a little less at low speed unless locked or spinning backwards.
    fn pilot_factor(&self, p: &TireParams, body_speed: f32) -> f32 {
        if p.front || self.brake_locked || self.av < 0.0 {
            return 1.0;
        }
        if self.integration == super::TireIntegration::Reference {
            let base = 0.85_f32;
            let factor =
                ((body_speed - crate::math::mph_to_ms(30.0)) * (1.0 / crate::math::mph_to_ms(20.0))).clamp(0.0, 1.0);
            return factor * (1.0 - base) + base;
        }
        let mph = ms_to_mph(body_speed);
        0.85 + 0.15 * ((mph - 30.0) / 20.0).clamp(0.0, 1.0)
    }

    /// Updates a tire pressed onto the ground and returns the lateral force (N, negative when the patch
    /// moves to the right). The longitudinal force is left in [`Tire::longitudinal_force`].
    pub fn update_loaded(&mut self, p: &TireParams, i: &LoadedInput) -> f32 {
        let reference = self.integration == super::TireIntegration::Reference;
        let r = p.radius;
        let dt = i.dt;
        let (fwd, lat) = (i.fwd_vel, i.lat_vel);

        // A wheel that was in the air starts rolling with the ground.
        if self.load <= 0.0 && !self.brake_locked {
            self.av = fwd / r;
            // Forget the speed and spin acceleration from before the wheel left the ground.
            if !reference {
                self.road_speed = fwd;
                self.angular_acc = 0.0;
            }
        }
        let fwd_acc = (fwd - self.road_speed) / dt;
        self.road_speed = fwd;
        self.load = i.load.max(0.0);
        self.lateral_speed = lat;
        let load = self.load;
        let body_mph = ms_to_mph(i.body_speed);

        // Brake torques.
        let bt = self.brake * p.brake_spec;
        let ebt = self.ebrake * p.ebrake_spec;
        let mut brake_torque = 0.0;
        if reference && self.brake_locked {
            self.drive_torque = 0.0;
        } else if fwd.abs() < 1.0 {
            // A viscous stop proportional to load and speed. Capped so a quarter of the body cannot be
            // pushed through zero speed in one step.
            let viscous = -(self.brake + self.ebrake) * load * fwd / r;
            let cap = i.quarter_mass * fwd.abs() / dt * r;
            brake_torque = match reference {
                true => viscous,
                false => viscous.clamp(-cap, cap),
            };
            self.drive_torque -= self.drive_torque * self.ebrake.clamp(0.0, 1.0);
        } else if !self.brake_locked {
            brake_torque = if self.av > 0.0 { -(bt + ebt) } else { bt + ebt };
        }
        let total_torque = self.drive_torque + brake_torque;

        // Slip.
        self.slip_angle = match reference {
            true => super::angle::reference_angle(lat, fwd.abs()),
            false => lat.atan2(fwd.abs()) / TAU,
        };
        let slip = self.av * r - fwd;
        self.slip = slip;
        let pilot = self.pilot_factor(p, i.body_speed);
        let (skid, patch_speed) = match reference {
            true => ((slip * slip + lat * lat).sqrt(), (fwd * fwd + lat * lat).sqrt()),
            false => (slip.hypot(lat), fwd.hypot(lat)),
        };
        let moving_patch = match reference {
            true => fwd != 0.0 || lat != 0.0,
            false => patch_speed > EPS,
        };
        let friction_active = skid > EPS && moving_patch;
        let (mut dyn_grip, mut ground_friction, mut ground_force) = (1.0, 0.0, 0.0);
        if friction_active {
            dyn_grip = p.dynamic_grip * self.traction_boost * pilot;
            let divisor = match reference {
                true => skid,
                false => skid.max(SKID_FLOOR),
            };
            ground_friction = match reference {
                true => (dyn_grip / divisor) * load,
                false => load * dyn_grip / divisor,
            };
            ground_force = fwd.abs() * load * dyn_grip / patch_speed;
        }

        // Brake lock test.
        let available = (self.brake * p.brake_lock_spec + self.ebrake * p.ebrake_spec) * LOCK_RATIO;
        let test_lock = !reference || friction_active;
        let locked = match (test_lock, available > ground_force * r + self.av.abs() * LOCK_AV_FACTOR) {
            (false, _) => self.brake_locked,
            (_, true) => {
                self.av = 0.0;
                available > 1.0
            }
            _ => false,
        };
        self.brake_locked = locked;

        // Sliding forces: full kinetic friction along the slip direction.
        let (mut slide_x, mut slide_y) = (ground_friction * slip, -ground_friction * lat);
        if body_mph < 1.0 && dyn_grip > 0.1 {
            let (x, y) = match reference {
                true => {
                    let inverse = 1.0 / dyn_grip;
                    (slide_x * inverse, slide_y * inverse)
                }
                false => (slide_x / dyn_grip, slide_y / dyn_grip),
            };
            slide_x = x;
            slide_y = y;
        }
        let reference_limit = reference && slide_x * total_torque > 0.0;
        if reference_limit || (!reference && !locked) {
            let limit = total_torque.abs() / r;
            slide_x = slide_x.clamp(-limit, limit);
        }
        // Gripping forces: the whole torque goes to the road, the lateral force follows the slip angle.
        let curve = lateral_force(load, self.slip_angle, p.grip_scale, self.grip_boost);
        let grip_y = if lat > 0.0 {
            -curve
        } else if lat < 0.0 {
            curve
        } else {
            0.0
        };
        let grip_x = total_torque / r + gripping_feedback(self.angular_acc, r, fwd_acc, reference);
        // The original picks one branch by last step's traction, which makes a tire at its limit flip
        // between the two every step. Blending by that traction settles on the friction limit instead.
        let w = match (locked, reference, self.traction >= 1.0) {
            (true, _, _) | (_, true, false) => 0.0,
            (_, true, true) => 1.0,
            _ => self.traction.clamp(0.0, 1.0),
        };
        let mut fx = w * grip_x + (1.0 - w) * slide_x;
        let mut fy = (w * grip_y + (1.0 - w) * slide_y) * self.lateral_boost;

        // Friction ellipse: the driven axis is longer while power is applied.
        let stretched = total_torque * fwd > 0.0 && !locked;
        if stretched {
            fx *= ELLIPSE_RATIO;
        }
        fy *= self.traction_circle.0;
        fx *= self.traction_circle.1;

        let len = match reference {
            true => (fx * fx + fy * fy).sqrt(),
            false => fx.hypot(fy),
        };
        let max_f = match reference {
            true => (((self.drift_friction * self.traction_boost) * p.static_grip) * load) * pilot,
            false => load * p.static_grip * self.traction_boost * self.drift_friction * pilot,
        };
        let mut max_slip = self.max_slip;
        if len > max_f && len > 0.001 {
            let ratio = max_f / len;
            self.traction = ratio;
            fx *= ratio;
            fy *= ratio;
            max_slip *= ratio * ratio;
        } else {
            self.traction = 1.0;
            if stretched {
                fx /= ELLIPSE_RATIO;
            }
        }
        if slip.abs() > max_slip {
            self.traction = scale_traction(self.traction, slip, max_slip, reference);
        }

        fy *= i.surface.lateral;
        fx *= i.surface.drive;

        // Cornering drag of a steered wheel.
        if fwd > 1.0 {
            fx -= (self.slip_angle * TAU).sin() * fy * self.drag_reduction / p.grip_scale.max(EPS);
        } else {
            fy *= lat.abs().min(1.0);
        }

        // Wheel spin.
        if locked {
            self.angular_acc = 0.0;
        } else {
            if self.traction < 1.0 {
                let torque = (total_torque - fx * r + self.last_torque) * 0.5;
                self.last_torque = torque;
                let eff = torque - self.av * ROLLING_FRICTION * i.surface.rolling;
                self.angular_acc = eff / WHEEL_INERTIA - self.traction * slip / (r * dt);
            }
            if !reference && self.traction >= 1.0 {
                // A gripping wheel follows the ground, and also sheds any slip left over from the last
                // step (a constant offset made the gripping and sliding branches alternate at the limit).
                self.angular_acc = fwd_acc / r - slip / (r * dt);
            } else {
                self.angular_acc = lerp(self.angular_acc, fwd_acc / r, self.traction);
            }
            self.av += self.angular_acc * dt;
            self.check_sign();
        }
        if reference && locked {
            self.check_sign();
        }

        self.longitudinal_force = fx;
        self.lateral_force = fy;
        fy
    }

    /// Updates a tire in the air: it only spins under brake torque and coasts otherwise.
    pub fn update_free(&mut self, p: &TireParams, dt: f32) {
        self.load = 0.0;
        self.slip = 0.0;
        self.traction = 0.0;
        self.slip_angle = 0.0;
        self.lateral_force = 0.0;
        self.longitudinal_force = 0.0;
        self.lateral_speed = 0.0;

        let available = (self.brake * p.brake_lock_spec + self.ebrake * p.ebrake_spec) * LOCK_RATIO;
        if available > self.av.abs() * LOCK_AV_FACTOR {
            self.av = 0.0;
            self.brake_locked = available > 1.0;
        } else {
            self.brake_locked = false;
        }
        if self.brake_locked {
            self.angular_acc = 0.0;
            return;
        }
        let bt = self.brake * p.brake_spec + self.ebrake * p.ebrake_spec;
        let brake_torque = if self.av > 0.0 { -bt } else { bt };
        self.angular_acc = (self.drive_torque + brake_torque) / WHEEL_INERTIA;
        self.av += self.angular_acc * dt;
        self.check_sign();
    }
}

#[cfg(test)]
mod pilot_tests {
    use super::*;
    use crate::{math::mph_to_ms, tires::TireIntegration};

    #[test]
    fn reference_gripping_feedback_divides_before_inertia() {
        let actual = gripping_feedback(1.25, 0.27, 274.4884, true);
        let stabilized = gripping_feedback(1.25, 0.27, 274.4884, false);
        assert_eq!(actual.to_bits(), 0xc61ea6f3);
        assert_ne!(actual.to_bits(), stabilized.to_bits());
    }

    #[test]
    fn reference_traction_divides_before_limit() {
        let actual = scale_traction(0.1, 0.71, 0.32, true);
        let stabilized = scale_traction(0.1, 0.71, 0.32, false);
        assert_eq!(actual.to_bits(), 0x3d389bc4);
        assert_ne!(actual.to_bits(), stabilized.to_bits());
    }

    #[test]
    fn reference_sliding_preserves_force_rounding() {
        for (dynamic_grip, traction_boost, static_grip, drift_friction, expected_x, expected_y) in [
            (1.2, 1.0, 100.0, 1.0, 0x44752529, 0xc19ce496),
            (1.1, 1.1, 100.0, 1.0, 0x4475252a, 0xc19ce496),
            (1.1, 1.0, 1.1, 0.85, 0x4442d442, 0xc179619d),
        ] {
            let mut tire = Tire {
                integration: TireIntegration::Reference,
                av: 2.0,
                load: 1000.0,
                traction: 0.5,
                road_speed: 0.1,
                last_sign: 1,
                traction_boost,
                drift_friction,
                ..Tire::default()
            };
            let params = TireParams {
                radius: 0.3,
                grip_scale: 1.0,
                static_grip,
                dynamic_grip,
                brake_spec: 0.0,
                brake_lock_spec: 0.0,
                ebrake_spec: 0.0,
                front: false,
            };
            let input = LoadedInput {
                lat_vel: 0.1,
                fwd_vel: 0.1,
                body_speed: 0.2,
                load: 1000.0,
                dt: 1.0 / 60.0,
                quarter_mass: 375.0,
                surface: crate::ground::SurfaceGrip::DEFAULT,
            };
            assert_eq!(tire.update_loaded(&params, &input).to_bits(), expected_y);
            assert_eq!(tire.longitudinal_force.to_bits(), expected_x);
        }
    }
    #[test]
    fn reference_pilot_curve_uses_the_si_threshold_and_span() {
        let mut t = Tire { integration: TireIntegration::Reference, av: 10.0, ..Tire::default() };
        let mut p = TireParams {
            radius: 0.3,
            grip_scale: 1.0,
            static_grip: 1.0,
            dynamic_grip: 1.0,
            brake_spec: 1.0,
            brake_lock_spec: 1.0,
            ebrake_spec: 1.0,
            front: false,
        };
        for (speed, expected) in
            [(0.0, 0.85), (mph_to_ms(30.0), 0.85), (mph_to_ms(40.0), 0.925), (mph_to_ms(50.0), 1.0)]
        {
            assert_eq!(t.pilot_factor(&p, speed), expected);
        }
        p.front = true;
        assert_eq!(t.pilot_factor(&p, 0.0), 1.0);
        p.front = false;
        t.brake_locked = true;
        assert_eq!(t.pilot_factor(&p, 0.0), 1.0);
        t.brake_locked = false;
        t.av = -10.0;
        assert_eq!(t.pilot_factor(&p, 0.0), 1.0);
    }
}
