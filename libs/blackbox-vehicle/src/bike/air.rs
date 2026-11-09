//! A bike that has left the ground: the player's air control, and the arcade help that makes it land on its wheels
//! (`docs/specs/air-control.md`, `docs/specs/ride6-bike-jumps.md`).

use glam::Vec3;

use crate::air_control::{AirAxes, AirControlSpec, Attitude, BodyFrame};
use crate::bike::class::TerrainClass;
use crate::bike::curve::ramp;
use crate::bike::{ArcadeAir, Bike, FRONT, REAR, lean_of, pitch_of};

/// The value of `AirState::touched` when no landing has happened (s): far beyond any window.
const NOT_LANDED: f32 = 1.0e3;
/// The seconds in the air before the bars start to centre, and over which they do (the lean controller turns them in the first moments).
const BARS_DELAY: f32 = 0.03;
const BARS_BLEND: f32 = 0.15;
/// The share of the tyre load cap (`ArcadeAir::load_cap_g`) under which the load is left alone; above it the load bends towards the cap.
const LOAD_KNEE: f32 = 0.6;
/// The share of the fall tests' airborne pitch and lean limits at which the player's push is gone (it fades from 0.5 of that).
const PUSH_LIMIT_SHARE: f32 = 0.7;
/// The most the fall tests' limits are widened to (rad): a landing or a loose road forgives a lot, never a bike on its side.
pub(in crate::bike) const MAX_LEAN: f32 = 1.48;
pub(in crate::bike) const MAX_PITCH: f32 = 1.40;
pub(in crate::bike) const MAX_HEADING: f32 = 1.50;

/// `limit` widened by `tolerance`, never past `cap` (and never narrower than it was): 1 leaves it as it is, exactly.
pub(in crate::bike) fn widen(limit: f32, tolerance: f32, cap: f32) -> f32 {
    (limit * tolerance).min(cap.max(limit))
}

/// What a bike keeps of its flights from step to step.
#[derive(Clone, Copy, Debug)]
pub struct AirState {
    /// The player's air control: `None` when it is off (the host's setting, or a bike the player does not ride), else the axes of this step.
    pub control: Option<AirAxes>,
    /// How long both wheels have been off the road without a break (s).
    pub air_time: f32,
    /// The seconds left in which the fall tests forgive a landing.
    pub grace: f32,
    /// The seconds since the last landing of a jump (`NOT_LANDED` while in the air or before one).
    pub touched: f32,
}

impl Default for AirState {
    fn default() -> Self {
        Self { control: None, air_time: 0.0, grace: 0.0, touched: NOT_LANDED }
    }
}

impl Bike {
    /// The player's air control for the next steps: the axes (pitch forward is the nose down, roll right the right side down, yaw right a
    /// turn to the right) with the control on, `None` with it off. It acts only while both wheels are off the road, and not on a fallen
    /// bike (`docs/specs/air-control.md`).
    pub fn set_air_control(&mut self, axes: Option<AirAxes>) {
        self.chassis.air.control = axes.map(AirAxes::clamped);
    }

    /// How long (s) both wheels have been off the road.
    pub fn air_time(&self) -> f32 {
        self.chassis.air.air_time
    }

    /// Whether the fall tests are forgiving a landing now.
    pub fn landing_forgiven(&self) -> bool {
        self.chassis.air.grace > 0.0
    }

    /// The timers of the flight and the bike's belief about the surface advance by one sub-step.
    pub(in crate::bike) fn advance_air_and_surface(&mut self, dt: f32) {
        let up = !self.wheels[FRONT].on_ground && !self.wheels[REAR].on_ground;
        let air = self.spec.steering.arcade.air;
        let state = &mut self.chassis.air;
        match up {
            true => {
                state.air_time += dt;
                // A wheel that touches for one step and bounces up again does not end the grace of a landing: only a new flight does.
                if state.air_time >= air.min_air_time {
                    state.grace = 0.0;
                    state.touched = NOT_LANDED;
                }
            }
            false => {
                if state.air_time >= air.min_air_time {
                    state.grace = air.landing_grace;
                    state.touched = 0.0;
                }
                state.air_time = 0.0;
                state.grace = (state.grace - dt).max(0.0);
                state.touched = (state.touched + dt).min(NOT_LANDED);
            }
        }
        self.chassis.rough.advance(self.spec.steering.arcade.rough.surface_rate, dt);
    }

    /// The factor on the fall tests' limits now: the landing's while it is forgiven, the loose ground's, whichever is wider. 1 in the
    /// reference steering.
    pub(in crate::bike) fn fall_tolerance(&self) -> f32 {
        let arcade = &self.spec.steering.arcade;
        let class = self.spec.terrain_class();
        let landing = match self.chassis.air.grace > 0.0 {
            true => arcade.air.tolerance(class),
            false => 1.0,
        };
        landing.max(arcade.rough.tolerance_factor(class.index(), self.chassis.rough.loose()))
    }

    /// The share of the lean controller's handlebar that is taken away because the bike is in the air (0 to 1): the bars centre.
    pub(in crate::bike) fn bars_centred_share(&self) -> f32 {
        let air = &self.spec.steering.arcade.air;
        let time = self.chassis.air.air_time;
        if air.handlebar_centre <= 0.0 || time <= 0.0 {
            return 0.0;
        }
        ramp(time, [BARS_DELAY, BARS_DELAY + BARS_BLEND]) * air.handlebar_centre.clamp(0.0, 1.0)
    }

    /// A tyre's load (N) held under `ArcadeAir::load_cap_g` times the bike's weight by a smooth curve: untouched below `LOAD_KNEE` of the cap,
    /// bending to it above (`knee + room tanh((load - knee) / room)`). No cap in the reference steering.
    pub(in crate::bike) fn soft_load(&self, load: f32) -> f32 {
        let cap_g = self.spec.steering.arcade.air.load_cap_g;
        if cap_g <= 0.0 {
            return load;
        }
        let cap = cap_g * self.body.mass() * 9.81;
        let knee = LOAD_KNEE * cap;
        if load <= knee {
            return load;
        }
        let room = cap - knee;
        knee + room * ((load - knee) / room).tanh()
    }

    /// The normal of the road under the wheels that touch it, `None` with none down.
    fn contact_normal(&self) -> Option<Vec3> {
        self.wheels
            .iter()
            .filter(|wheel| wheel.on_ground)
            .fold(Vec3::ZERO, |sum, wheel| sum + wheel.normal)
            .try_normalize()
    }

    /// The normal of the road under the wheels in the air (the world's up when none is in reach).
    fn landing_normal(&self) -> Vec3 {
        self.wheels.iter().fold(Vec3::ZERO, |sum, wheel| sum + wheel.below).try_normalize().unwrap_or(Vec3::Y)
    }

    /// The landing assist (`ArcadeAir::landing_accel`): for `landing_window` seconds after the landing of a jump, an extra deceleration along
    /// the road's normal, proportional to the speed into the road above `landing_soft_speed` and held to `landing_accel` times the class's
    /// factor, acting at the centre of gravity (no torque). It takes away speed the suspension and the tyres could not, and it is gone when the
    /// bike no longer moves into the road.
    pub(in crate::bike) fn apply_landing_assist(&mut self) {
        let air = self.spec.steering.arcade.air;
        if air.landing_accel <= 0.0 || self.chassis.air.touched > air.landing_window {
            return;
        }
        let Some(normal) = self.contact_normal() else { return };
        let excess = -self.body.linear_velocity.dot(normal) - air.landing_soft_speed;
        if excess <= 0.0 {
            return;
        }
        let limit = air.landing_accel * air.factor(self.spec.terrain_class());
        let acceleration = (air.landing_gain * excess).min(limit);
        self.body.apply_force(normal * (acceleration * self.body.mass()));
    }

    /// The torque of the levelling (`ArcadeAir::level_*`): the pitch and the roll turn the bike's up towards the normal of the road below, a
    /// spring and a damper held to `level_accel` times the class's factor, coming in over `level_delay`. An axis the player is pushing is left
    /// to him by the share he pushes.
    fn level_torque(&self, air: &ArcadeAir, class: TerrainClass, frame: &BodyFrame, input: AirAxes) -> Vec3 {
        let weight = ramp(self.chassis.air.air_time, air.level_delay);
        if air.level_accel <= 0.0 || weight <= 0.0 {
            return Vec3::ZERO;
        }
        let transpose = frame.rotation.transpose();
        let up = frame.rotation * Vec3::Y;
        let error = transpose * up.cross(self.landing_normal());
        let omega = transpose * frame.angular_velocity;
        let bound = air.level_accel * air.factor(class);
        let acceleration = |error: f32, omega: f32, pushed: f32| {
            ((air.level_gain * error - air.level_damping * omega) * (1.0 - pushed.abs().min(1.0))).clamp(-bound, bound)
        };
        let local =
            Vec3::new(acceleration(error.x, omega.x, input.pitch), 0.0, acceleration(error.z, omega.z, input.roll));
        frame.rotation * (local * (weight * frame.inertia))
    }

    /// The torque of the air: the arcade levelling and the player's air control, with both wheels off the road and the rider on the bike.
    pub(in crate::bike) fn apply_air_attitude(&mut self) {
        if self.departure.off || self.wheels[FRONT].on_ground || self.wheels[REAR].on_ground {
            return;
        }
        let air = self.spec.steering.arcade.air;
        let class = self.spec.terrain_class();
        let rotation = self.body.rotation();
        let frame = BodyFrame { rotation, angular_velocity: self.body.angular_velocity, inertia: self.body.inertia() };
        let control = self.chassis.air.control;
        let mut torque = self.level_torque(&air, class, &frame, control.unwrap_or(AirAxes::ZERO));
        if let Some(axes) = control {
            let extra = self.spec.fall.extra;
            let pick = |airborne: f32, ground: f32| match airborne > 0.0 {
                true => airborne,
                false => ground,
            };
            let attitude = Attitude {
                pitch_up: pitch_of(rotation),
                lean_right: lean_of(rotation),
                pitch_limit: pick(extra.airborne_pitch, self.spec.fall.pitch) * PUSH_LIMIT_SHARE,
                lean_limit: pick(extra.airborne_lean, self.spec.fall.lean) * PUSH_LIMIT_SHARE,
            };
            torque += AirControlSpec::BIKE.torque(axes, &frame, &attitude);
        }
        if torque == Vec3::ZERO {
            return;
        }
        self.body.apply_torque(torque);
    }
}
