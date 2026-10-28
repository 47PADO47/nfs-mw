//! The engine, the clutch and the gearbox of a bike, turned into a torque at the rear wheel (`docs/specs/ride6-engine-drivetrain.md`).
//!
//! The engine has a speed of its own and an inertia. The torque is the blend of the drag of the closed throttle and the torque
//! of the curve; the rev limiter cuts the throttle in turn. A clutch joins the engine to the rear wheel: locked, the two turn
//! together (the engine's inertia is added to the wheel's); slipping, the clutch passes its capacity and the engine is free.
//! The gears change automatically (the decision is a choice, see `choices.rs`).

use std::f32::consts::TAU;

use super::choices;
use super::curve::{curve, ramp};
use super::gearbox::{Gearbox, peak_torque_rpm};
use super::spec::{EngineSpec, TransmissionSpec};

/// What the powertrain asks of the rear wheel for this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drive {
    /// The torque on the wheel (N m, negative for engine braking).
    pub torque: f32,
    /// The inertia (kg m^2) the engine adds to the wheel's through a locked clutch.
    pub extra_inertia: f32,
}

/// The drive side of the bike.
#[derive(Clone, Debug)]
pub struct Powertrain {
    gear: usize,
    /// Engine speed (rad/s).
    speed: f32,
    shift_timer: f32,
    /// The engine speed of the peak of the torque curve (rpm).
    peak_rpm: f32,
    locked: bool,
    /// How far the clutch has come back after a gear change, 0 to 1.
    engage: f32,
    slipper: f32,
    /// The torque the clutch passed to the wheel in the last step, on the engine side (N m).
    transmitted: f32,
    limiter_timer: f32,
    limiter_cut: bool,
    /// Whether the change under way is a change down (it blips the throttle).
    blip: bool,
    pending: Option<Pending>,
    gearbox: Gearbox,
    /// How long the feathering controller has been acting (s); 0 when it is not.
    feather_timer: f32,
    /// Whether a start from a standstill is under way (the launch clutch acts).
    starting: bool,
}

/// What the wheel's solve needs to hand back to the powertrain.
#[derive(Clone, Copy, Debug)]
struct Pending {
    ratio: f32,
    torque: f32,
    capacity: f32,
    locked: bool,
}

fn rpm_to_rad(rpm: f32) -> f32 {
    rpm * TAU / 60.0
}

fn rad_to_rpm(speed: f32) -> f32 {
    speed * 60.0 / TAU
}

/// The sign of `x` smoothed over `width`, so a clutch near equal speeds does not flip its torque each step.
fn smooth_sign(x: f32, width: f32) -> f32 {
    x / (x.abs() + width)
}

impl Powertrain {
    pub fn new(engine: &EngineSpec, transmission: &TransmissionSpec) -> Self {
        Self {
            gear: 0,
            speed: rpm_to_rad(engine.idle_rpm),
            shift_timer: 0.0,
            peak_rpm: peak_torque_rpm(engine),
            locked: false,
            engage: 1.0,
            slipper: 0.0,
            transmitted: 0.0,
            limiter_timer: 0.0,
            limiter_cut: false,
            blip: false,
            pending: None,
            gearbox: Gearbox::generate(engine, transmission),
            feather_timer: 0.0,
            starting: false,
        }
    }

    /// The gear in use, 1 for first.
    pub fn gear(&self) -> usize {
        self.gear + 1
    }

    pub fn rpm(&self) -> f32 {
        rad_to_rpm(self.speed)
    }

    /// Whether a gear change is under way (the clutch is open for it).
    pub fn shifting(&self) -> bool {
        self.shift_timer > 0.0
    }

    /// The ratio from the engine to the rear wheel in gear index `gear`.
    fn ratio(transmission: &TransmissionSpec, gear: usize) -> f32 {
        transmission.gears.get(gear).copied().unwrap_or(1.0) * transmission.final_ratio
    }

    /// Match the engine and the gear to a road speed (the bike starts already moving) on a rear wheel of radius `radius`.
    pub fn match_speed(&mut self, engine: &EngineSpec, transmission: &TransmissionSpec, wheel_spin: f32, radius: f32) {
        self.gear = self.gearbox.gear_for_speed(engine, transmission, wheel_spin * radius, radius);
        let coupled = wheel_rpm(transmission, self.gear, wheel_spin);
        self.speed = rpm_to_rad(coupled.max(engine.idle_rpm));
        self.locked = coupled >= engine.idle_rpm;
        self.engage = 1.0;
        self.shift_timer = 0.0;
        self.slipper = 0.0;
        self.transmitted = 0.0;
        self.pending = None;
        self.feather_timer = 0.0;
        self.starting = false;
    }

    /// The engine's torque at the crankshaft (N m) for a throttle, at its present speed.
    fn engine_torque(&self, engine: &EngineSpec, throttle: f32) -> f32 {
        let rpm = self.rpm();
        let peak = engine.peak_torque * curve(&engine.torque_curve, rpm);
        let share = (rpm / engine.max_rpm).max(0.0);
        let drag_factor = match engine.friction_curve.is_empty() {
            true => share,
            false => curve(&engine.friction_curve, share),
        };
        let drag = engine.friction_torque * drag_factor;
        let closed = -drag;
        closed + (peak - closed) * throttle
    }

    /// The traction control scales the throttle about the zero-torque point (`ride6-engine-drivetrain.md`, "Engine torque").
    fn traction_control(&self, engine: &EngineSpec, throttle: f32, cut: f32) -> f32 {
        let rpm = self.rpm();
        let peak = engine.peak_torque * curve(&engine.torque_curve, rpm);
        let share = (rpm / engine.max_rpm).max(0.0);
        let factor = match engine.friction_curve.is_empty() {
            true => share,
            false => curve(&engine.friction_curve, share),
        };
        let drag = engine.friction_torque * factor;
        let zero = match peak + drag > 0.0 {
            true => drag / (peak + drag),
            false => 1.0,
        };
        match throttle > zero {
            true => zero + (throttle - zero) * (1.0 - cut.clamp(0.0, 1.0)),
            false => throttle,
        }
    }

    /// The throttle the engine sees: the rider's, the floor below the idle speed, the blip of a change down and the cut of
    /// the rev limiter.
    fn throttle(&mut self, engine: &EngineSpec, transmission: &TransmissionSpec, input: f32, dt: f32) -> f32 {
        let mut throttle = input.clamp(0.0, 1.0);
        if self.rpm() < engine.idle_rpm {
            throttle = throttle.max(engine.throttle_at_idle);
        }
        if self.shifting() {
            throttle = throttle.max(transmission.blip_throttle * f32::from(self.blip));
        }
        if self.rpm() > engine.max_rpm {
            self.limiter_timer += dt;
            if self.limiter_timer >= engine.limiter_cut_time {
                self.limiter_timer = 0.0;
                self.limiter_cut = !self.limiter_cut;
            }
            return throttle * f32::from(!self.limiter_cut);
        }
        self.limiter_timer = 0.0;
        self.limiter_cut = false;
        throttle
    }
}

/// The engine speed (rpm) that a rear wheel turning at `spin` rad/s forces in gear index `gear`.
fn wheel_rpm(transmission: &TransmissionSpec, gear: usize, spin: f32) -> f32 {
    rad_to_rpm(spin * Powertrain::ratio(transmission, gear))
}

/// What a step of the powertrain is given.
#[derive(Clone, Copy, Debug)]
pub struct DriveInput {
    pub throttle: f32,
    /// The spin of the rear wheel (rad/s).
    pub wheel_spin: f32,
    /// The road speed (m/s).
    pub speed: f32,
    /// The effective radius of the rear wheel (m) and the lean as a share of the lean limit (0 upright, 1 at the limit).
    pub radius: f32,
    pub lean: f32,
    /// How much the traction control cuts the throttle, 0 to 1.
    pub traction_cut: f32,
    pub dt: f32,
}

impl Powertrain {
    /// The first half of a step: the torque and the inertia the engine puts on the rear wheel. Call [`Powertrain::finish`]
    /// with the spin the wheel's solve gives.
    pub fn drive(&mut self, engine: &EngineSpec, transmission: &TransmissionSpec, input: &DriveInput) -> Drive {
        let dt = input.dt;
        self.shift_timer = (self.shift_timer - dt).max(0.0);
        if self.shift_timer > 0.0 {
            self.engage = 0.0;
        }
        if self.shift_timer <= 0.0 {
            self.engage = (self.engage + dt / choices::CLUTCH_ENGAGE_TIME).min(1.0);
        }
        self.change_gear(engine, transmission, input);
        let ratio = Self::ratio(transmission, self.gear);
        let coupled_speed = (input.wheel_spin * ratio).max(0.0);
        let throttle = self.throttle(engine, transmission, input.throttle, dt);
        let throttle = self.traction_control(engine, throttle, input.traction_cut);
        let torque = self.engine_torque(engine, throttle);
        let engagement = self.engagement(engine, transmission, input, torque, rad_to_rpm(coupled_speed));
        let capacity = engagement * transmission.clutch_max_factor * engine.peak_torque;
        if capacity <= 0.0 {
            self.locked = false;
        }
        let efficiency = transmission.efficiency;
        if self.locked {
            self.pending = Some(Pending { ratio, torque, capacity, locked: true });
            return Drive { torque: torque * ratio * efficiency, extra_inertia: engine.moi * ratio * ratio };
        }
        let difference = self.speed - coupled_speed;
        let passed = capacity * smooth_sign(difference, 1.0);
        let before = self.speed;
        self.speed = (self.speed + dt * (torque - passed) / engine.moi.max(1e-4)).max(0.0);
        if (before - coupled_speed) * (self.speed - coupled_speed) < 0.0 && capacity > 0.0 {
            self.speed = coupled_speed;
            self.locked = true;
        }
        self.transmitted = passed;
        self.pending = Some(Pending { ratio, torque, capacity, locked: false });
        Drive { torque: passed * ratio * efficiency, extra_inertia: 0.0 }
    }

    /// The second half: the rear wheel has spun to `wheel_spin`. A locked engine follows it.
    pub fn finish(&mut self, engine: &EngineSpec, wheel_spin: f32, dt: f32) {
        let Some(pending) = self.pending.take() else { return };
        if !pending.locked {
            return;
        }
        let new_speed = wheel_spin * pending.ratio;
        if new_speed < 0.0 {
            self.speed = 0.0;
            self.locked = false;
            return;
        }
        let passed = pending.torque - engine.moi * (new_speed - self.speed) / dt.max(1e-6);
        self.speed = new_speed;
        self.transmitted = passed;
        if passed.abs() > pending.capacity + 1e-3 {
            self.locked = false;
        }
    }

    /// The clutch's engagement, 0 (open) to 1 (locked): the rider's lever (open for a change), the launch, the slipper and
    /// the return after a change. `torque` is the engine's torque now and `coupled_rpm` the engine speed the wheel forces.
    fn engagement(
        &mut self,
        engine: &EngineSpec,
        transmission: &TransmissionSpec,
        input: &DriveInput,
        torque: f32,
        coupled_rpm: f32,
    ) -> f32 {
        let mut lever: f32 = 0.0;
        if self.shifting() && choices::CLUTCH_OPEN_DURING_SHIFT {
            lever = 1.0;
        }
        lever = lever.max(1.0 - self.engage);
        lever = lever.max(self.launch(engine, transmission, input, lever, torque, coupled_rpm));
        let back = (-self.transmitted).max(0.0);
        let target = ramp(back, transmission.slipper_range) * transmission.slipper_intensity;
        self.slipper = target.max(self.slipper - choices::SLIPPER_FALL_RATE * input.dt);
        let open = lever.max(self.slipper).clamp(0.0, 1.0);
        (1.0 - open).powf(transmission.clutch_linearity.max(0.01))
    }

    /// The clutch input of a start: the feathering controller when the data has one (`feather_factor` above 0), else the plain
    /// launch clutch that comes in with the engine speed and goes with the road speed (`ride6-gearbox-auto.md`, section 3).
    fn launch(
        &mut self,
        engine: &EngineSpec,
        transmission: &TransmissionSpec,
        input: &DriveInput,
        clutch: f32,
        torque: f32,
        coupled_rpm: f32,
    ) -> f32 {
        let kmh = input.speed * 3.6;
        // Without the throttle the clutch is pulled and the start is over: at a standstill that holds the engine at its idle,
        // and a start that is let go of (`ride6-gearbox-auto.md`: the clutch goes to 1 and the starting sequence is reset).
        if input.throttle <= 0.0 && (self.starting || input.speed < choices::START_SPEED) {
            self.starting = false;
            self.feather_timer = 0.0;
            return 1.0;
        }
        if input.throttle > 0.0 && input.speed < choices::START_SPEED {
            self.starting = true;
        }
        if !self.starting || (transmission.release_speed > 0.0 && input.speed >= transmission.release_speed) {
            self.starting = false;
            self.feather_timer = 0.0;
            return 0.0;
        }
        if transmission.feather_factor <= 0.0 {
            let [low, high] = transmission.launch_rpm;
            let by_engine = 1.0 - ramp(self.rpm(), [low * engine.max_rpm, high * engine.max_rpm]);
            let by_speed = match transmission.release_speed > 0.0 {
                true => 1.0 - ramp(kmh, [0.0, transmission.release_speed * 3.6]),
                false => 1.0,
            };
            return by_engine * by_speed;
        }
        let f = &transmission.feather;
        let reach =
            (engine.max_rpm * transmission.feather_max_factor + self.peak_rpm * transmission.feather_factor).max(1.0);
        let slip = (self.rpm() - coupled_rpm) / reach;
        let normalised = self.rpm() / reach;
        let wanted = slip > f.min_slip || normalised <= f.target_tolerance || self.feather_timer > 0.0;
        if input.throttle <= 0.0 || clutch >= 1.0 || !wanted {
            self.feather_timer = 0.0;
            return 0.0;
        }
        let reference = coupled_rpm.max(engine.idle_rpm);
        if self.feather_timer > 0.3 && slip < 0.01 && reference / reach >= f.exit_tolerance {
            self.feather_timer = 0.0;
            return 0.0;
        }
        self.feather_timer = self.feather_timer.max(1e-3) + input.dt;
        // The clutch torque that lets the engine gain `target` rad/s over the correction step, as a share of the most the
        // clutch carries, turned into an input through the linearity.
        let capacity = (transmission.clutch_max_factor * engine.peak_torque).max(1e-6);
        let step = f.delta_time.max(1e-4);
        let target = (reach - self.rpm()) * f.rpm_raise * TAU / 60.0;
        let after = self.speed + step * torque / engine.moi.max(1e-4);
        let coupled = rpm_to_rad(coupled_rpm);
        let needed = (after - coupled).signum() * (step * torque - engine.moi * target) / (capacity * step);
        let request = 1.0 - needed.max(0.0).powf(1.0 / transmission.clutch_linearity.max(0.01));
        let release = (1.0 - reference * f.release_factor / reach).clamp(0.0, 1.0);
        (release * request).clamp(0.0, 1.0)
    }

    /// The automatic gearbox: change up when the road speed passes the speed at which the engine would reach the change-up
    /// point in the present gear, down when it falls below that of the change-down point in the gear before
    /// (`ride6-gearbox-auto.md`, section 2). Lean holds the changes back, the throttle holds the change down back.
    fn change_gear(&mut self, engine: &EngineSpec, transmission: &TransmissionSpec, input: &DriveInput) {
        if self.shifting() || self.feather_timer > 0.0 {
            return;
        }
        let lean = input.lean.clamp(0.0, 1.0);
        let held = |pair: [f32; 2], value: f32| pair[0] * value.powf(pair[1]);
        let hold_up = held(transmission.prevent_up_lean, lean);
        let hold_down = (held(transmission.prevent_down_lean, lean)
            + held(transmission.prevent_down_throttle, input.throttle.clamp(0.0, 1.0)))
        .clamp(0.0, 1.0);
        let speed = input.speed.abs();
        if self.gearbox.wants_up(engine, transmission, self.gear, speed, input.radius, hold_up) {
            self.gear += 1;
            self.shift_timer = transmission.up_shift_time;
            self.blip = false;
            return;
        }
        if self.gearbox.wants_down(engine, transmission, self.gear, speed, input.radius, hold_down) {
            self.gear -= 1;
            self.shift_timer = transmission.down_shift_time;
            self.blip = true;
        }
    }
}

#[cfg(test)]
#[path = "powertrain_tests.rs"]
mod tests;
