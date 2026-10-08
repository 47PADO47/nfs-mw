use super::gearbox::{ShiftPoints, ShiftPotential};
use super::spec::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE, TransmissionSpec};
use crate::engine::{Clutch, EngineSpec};
use crate::induction::{Induction, InductionInput, InductionSpec};
use crate::math::{clamp_signed, mph_to_ms, rad_to_rpm, ramp, rpm_to_rad};
use crate::nos::{Nos, NosInput, NosSpec};

/// Seconds a manual shift in automatic mode holds the automatic logic off.
const SPORT_HOLD: f32 = 1.25;

/// Inputs of one powertrain step.
pub struct TickInput<'a> {
    pub dt: f32,
    /// Gas pedal, 0..1.
    pub gas: f32,
    pub nos_held: bool,
    /// Engine blown (throttle 0, no nitrous).
    pub blown: bool,
    /// The gearbox shifts by itself.
    pub automatic: bool,
    /// Body speed (m/s).
    pub speed: f32,
    /// Wheel angular velocities (front left, front right, rear left, rear right), written back by the
    /// lock-up and the rev limiter.
    pub wheel_av: &'a mut [f32; 4],
    pub grounded: [bool; 4],
    /// Mean slip of the driven wheels in the direction of travel (m/s, + = spinning).
    pub drive_slip: f32,
    /// Largest slip magnitude over the four wheels (m/s): gates automatic up-shifts.
    pub max_wheel_slip: f32,
    /// Tuning sliders in [-1, 1].
    pub induction_tuning: f32,
    pub nos_tuning: f32,
}

/// Engine, clutch, gearbox, induction and nitrous as one unit: turns the pedal into drive torque at the
/// wheels and reads the wheel speeds back one step later.
#[derive(Clone, Debug)]
pub struct Powertrain {
    pub(super) engine: EngineSpec,
    pub(super) trans: TransmissionSpec,
    induction_spec: InductionSpec,
    nos_spec: NosSpec,
    avg_wheel_radius: f32,
    /// Engine angular velocity (rad/s).
    pub(super) omega: f32,
    /// What the engine speed would be if locked to the wheels (rad/s).
    pub(super) omega_trans: f32,
    pub(super) rpm_display: f32,
    pub clutch: Clutch,
    pub(super) gear: usize,
    pub(super) shift_timer: f32,
    sport_timer: f32,
    pub(super) throttle: f32,
    pub(super) shift_points: ShiftPoints,
    pub(super) peak_torque_rpm: f32,
    pub(super) peak_torque_nm: f32,
    pub induction: Induction,
    pub nos: Nos,
    pub(super) prev_rpm_diff: f32,
    pub(super) drive_torque: f32,
    pub(super) engine_braking: bool,
}

impl Powertrain {
    /// `avg_wheel_radius` (m) is only used for the speedometer and shift logic.
    pub fn new(
        engine: EngineSpec,
        trans: TransmissionSpec,
        induction_spec: InductionSpec,
        nos_spec: NosSpec,
        avg_wheel_radius: f32,
    ) -> Self {
        let shift_points = ShiftPoints::compute(&engine, &trans);
        let (peak_torque_nm, peak_torque_rpm) = Self::scan_peak(&engine, &induction_spec);
        let idle = engine.idle_rad();
        let nos = Nos::new(&nos_spec);
        Self {
            avg_wheel_radius,
            omega: idle,
            omega_trans: 0.0,
            rpm_display: engine.idle,
            clutch: Clutch::default(),
            gear: GEAR_FIRST.min(trans.top_gear()),
            shift_timer: 0.0,
            sport_timer: 0.0,
            throttle: 0.0,
            shift_points,
            peak_torque_rpm,
            peak_torque_nm,
            induction: Induction::default(),
            nos,
            prev_rpm_diff: 0.0,
            drive_torque: 0.0,
            engine_braking: false,
            engine,
            trans,
            induction_spec,
            nos_spec,
        }
    }

    fn scan_peak(engine: &EngineSpec, induction: &InductionSpec) -> (f32, f32) {
        let (mut best, mut best_rpm) = (0.0_f32, engine.idle);
        let mut rpm = engine.idle;
        while rpm <= engine.red_line {
            let t = engine.torque_nm(rpm) * (1.0 + induction.full_boost(rpm, engine.idle, engine.red_line));
            if t > best {
                best = t;
                best_rpm = rpm;
            }
            rpm += 50.0;
        }
        (best, best_rpm)
    }

    /// Resets to the spawn state: idle, clutch engaged, first gear, nothing spooled.
    pub fn reset(&mut self) {
        self.omega = self.engine.idle_rad();
        self.omega_trans = 0.0;
        self.rpm_display = self.engine.idle;
        self.clutch.reset();
        self.gear = GEAR_FIRST.min(self.trans.top_gear());
        self.shift_timer = 0.0;
        self.sport_timer = 0.0;
        self.throttle = 0.0;
        self.induction = Induction::default();
        self.nos = Nos::new(&self.nos_spec);
        self.drive_torque = 0.0;
    }

    pub fn engine_spec(&self) -> &EngineSpec {
        &self.engine
    }

    pub fn transmission_spec(&self) -> &TransmissionSpec {
        &self.trans
    }

    /// Current gear id (0 reverse, 1 neutral, 2 first, ...).
    pub fn gear(&self) -> usize {
        self.gear
    }

    pub fn top_gear(&self) -> usize {
        self.trans.top_gear()
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }

    /// Engine speed in rpm.
    pub fn rpm(&self) -> f32 {
        rad_to_rpm(self.omega)
    }

    /// Smoothed engine speed for a tachometer or engine sound.
    pub fn rpm_display(&self) -> f32 {
        self.rpm_display
    }

    /// Drive torque handed to the chassis last step (N m at the wheels, gear ratio included).
    pub fn drive_torque(&self) -> f32 {
        self.drive_torque
    }

    /// Peak engine torque (N m, with full induction boost) and its rpm.
    pub fn peak_torque(&self) -> (f32, f32) {
        (self.peak_torque_nm, self.peak_torque_rpm)
    }

    pub fn shift_points(&self) -> &ShiftPoints {
        &self.shift_points
    }

    /// True while a gear change is in progress.
    pub fn shifting(&self) -> bool {
        self.shift_timer > 0.0
    }

    /// True when the torque is currently negative (engine braking).
    pub fn is_engine_braking(&self) -> bool {
        self.engine_braking
    }

    /// Road speed (m/s) at `rpm` in `gear`, without the speed limiter.
    pub fn speed_at(&self, rpm: f32, gear: usize) -> f32 {
        let ratio = self.trans.ratio(gear) * self.trans.final_gear;
        if ratio <= 1e-6 || self.engine.red_line <= self.engine.idle {
            return 0.0;
        }
        let clutch_rpm =
            (rpm - self.engine.idle) / ratio / (self.engine.red_line - self.engine.idle) * self.engine.red_line;
        (rpm_to_rad(clutch_rpm) * self.avg_wheel_radius).max(0.0)
    }

    /// Road speed implied by the transmission-side engine speed, clamped to the speed limiter.
    pub fn speedometer(&self) -> f32 {
        let v = self.speed_at(rad_to_rpm(self.omega_trans), self.gear);
        match self.engine.speed_limiter[0] {
            l if l > 0.0 => v.min(mph_to_ms(l)),
            _ => v,
        }
    }

    /// Top speed: the red line in the top gear, clamped to the limiter.
    pub fn max_speed(&self) -> f32 {
        let v = self.speed_at(self.engine.red_line, self.trans.top_gear());
        match self.engine.speed_limiter[0] {
            l if l > 0.0 => v.min(mph_to_ms(l)),
            _ => v,
        }
    }

    /// Changes gear: the shift delay is `SHIFT_SPEED * ratio` (a quarter of it for down-shifts) and the
    /// clutch opens. Returns false if the gear is out of range or unchanged.
    pub fn shift(&mut self, new_gear: usize) -> bool {
        if new_gear > self.trans.top_gear() || new_gear == self.gear {
            return false;
        }
        let delay = self.trans.shift_speed * self.trans.ratio(new_gear);
        self.shift_timer = if new_gear < self.gear { delay * 0.25 } else { delay };
        self.gear = new_gear;
        self.clutch.disengage();
        true
    }

    /// Manual shift request by one gear (`dir` = +1 or -1), as the shift buttons do. In automatic mode this
    /// is a "sport shift" that holds the automatic logic off for a while. Reverse is ignored.
    pub fn request_shift(&mut self, dir: i32, automatic: bool) {
        if self.gear == GEAR_REVERSE {
            return;
        }
        let desired = (self.gear as i64 + dir as i64).clamp(1, self.trans.top_gear() as i64) as usize;
        if desired == self.gear {
            return;
        }
        if !automatic {
            self.shift(desired);
            return;
        }
        if self.gear <= GEAR_NEUTRAL || self.shifting() {
            return;
        }
        let rpm = rad_to_rpm(self.omega_trans);
        let wish = self.shift_points.potential(&self.engine, &self.trans, self.gear, rpm, self.throttle);
        let contradicts = (dir > 0 && wish == ShiftPotential::Down) || (dir < 0 && wish == ShiftPotential::Up);
        if !contradicts && self.shift(desired) {
            self.sport_timer = SPORT_HOLD;
        }
    }

    /// Teleport or respawn at road speed `v` (m/s, negative = backwards): picks a gear and a matching rpm.
    pub fn match_speed(&mut self, v: f32) {
        self.reset();
        if v < 0.0 {
            self.gear = GEAR_REVERSE;
        } else {
            let top = self.trans.top_gear();
            let mut g = GEAR_FIRST.min(top);
            while g < top {
                let rpm = self.speed_to_rpm(v, g);
                if rpm < self.shift_points.up[g] {
                    break;
                }
                g += 1;
            }
            self.gear = g;
        }
        let rpm = self.speed_to_rpm(v.abs(), self.gear).clamp(self.engine.idle, self.engine.red_line);
        self.omega = rpm_to_rad(rpm);
        self.omega_trans = self.omega;
        self.rpm_display = rpm;
    }

    fn speed_to_rpm(&self, v: f32, gear: usize) -> f32 {
        let (min_w, max_w) = (self.engine.idle_rad(), self.engine.red_line_rad());
        let ratio = self.trans.ratio(gear) * self.trans.final_gear;
        let wheel_w = v / self.avg_wheel_radius.max(1e-3);
        rad_to_rpm(min_w + wheel_w * ratio * (max_w - min_w) / max_w)
    }

    /// Advances throttle, nitrous, speed limiter, induction, shifting and the torque loop by one step and
    /// returns the drive torque for the chassis (N m at the wheels).
    pub fn tick(&mut self, i: &mut TickInput<'_>) -> f32 {
        let dt = i.dt;
        self.throttle = if i.blown { 0.0 } else { i.gas.clamp(0.0, 1.0) };

        let speed_mph = crate::math::ms_to_mph(i.speed);
        self.nos.update(
            &self.nos_spec,
            &NosInput {
                dt,
                held: i.nos_held,
                gear: self.gear,
                throttle: self.throttle,
                speed_mph,
                blown: i.blown,
                tuning: i.nos_tuning,
            },
        );

        let [limit, band] = self.engine.speed_limiter;
        if self.gear > GEAR_NEUTRAL && limit > 0.0 && band > 0.0 {
            let v = self.speed_at(rad_to_rpm(self.omega_trans), self.gear);
            let lim = mph_to_ms(limit);
            if v > lim {
                self.throttle *= 1.0 - ((v - lim) / mph_to_ms(band)).clamp(0.0, 1.0);
            }
        }

        let rpm = rad_to_rpm(self.omega);
        self.induction.update(
            &self.induction_spec,
            &InductionInput {
                dt,
                throttle: self.throttle,
                rpm,
                idle: self.engine.idle,
                red_line: self.engine.red_line,
                shifting: self.shifting(),
                tuning: i.induction_tuning,
            },
        );

        self.shift_timer = (self.shift_timer - dt).max(0.0);
        self.clutch.update(dt);
        if i.automatic {
            self.auto_shift(i, dt);
        }
        self.command_clutch();

        let mut ctx = super::torque_loop::LoopCtx {
            dt,
            av: i.wheel_av,
            grounded: i.grounded,
            drive_slip: i.drive_slip,
            speed: i.speed,
        };
        self.drive_torque = clamp_signed(self.torque_loop(&mut ctx), 1.0e6);
        self.smooth_rpm(dt);
        self.drive_torque
    }

    fn auto_shift(&mut self, i: &TickInput<'_>, dt: f32) {
        if self.gear == GEAR_REVERSE || self.shifting() {
            return;
        }
        if self.sport_timer > 0.0 {
            self.sport_timer -= dt * (2.0 - i.gas.clamp(0.0, 1.0));
            return;
        }
        if self.gear == GEAR_NEUTRAL {
            self.shift(GEAR_FIRST.min(self.trans.top_gear()));
            return;
        }
        if !self.clutch.is_engaged() {
            return;
        }
        let rpm = rad_to_rpm(self.omega_trans);
        match self.shift_points.potential(&self.engine, &self.trans, self.gear, rpm, self.throttle) {
            ShiftPotential::Down => {
                let target = self.shift_points.downshift_target(&self.trans, self.gear, rpm);
                self.shift(target);
            }
            ShiftPotential::Up => {
                if i.grounded.iter().all(|g| *g) && i.max_wheel_slip < 4.0 {
                    self.shift(self.gear + 1);
                }
            }
            ShiftPotential::None => {}
        }
    }

    fn command_clutch(&mut self) {
        let idle = self.engine.idle_rad();
        match self.gear {
            GEAR_NEUTRAL => self.clutch.disengage(),
            GEAR_FIRST | GEAR_REVERSE => {
                let extra = if self.gear == GEAR_REVERSE { 2400.0 } else { 800.0 };
                let limit = idle + rpm_to_rad(extra);
                if self.omega > limit || self.omega_trans > limit || self.throttle >= 0.1 {
                    self.clutch.engage(0.05);
                } else {
                    self.clutch.disengage();
                }
            }
            _ => self.clutch.engage(0.25),
        }
    }

    fn smooth_rpm(&mut self, dt: f32) {
        let inertia = self.engine.inertia(self.gear == GEAR_NEUTRAL);
        let fast = (self.shifting() && self.gear > GEAR_FIRST) || self.gear == GEAR_NEUTRAL;
        let max_decel = -(if fast { 15.0 } else { 2.5 }) * 1000.0 / inertia;
        let new = rad_to_rpm(self.omega);
        let old = self.rpm_display;
        let mut n = new;
        if (new - old) / dt < max_decel {
            n = (old + max_decel * dt).max(new);
        }
        self.rpm_display = 0.55 * n + 0.45 * old;
    }

    /// Fraction 0..1 of how far the rpm is between idle and red line (for gauges).
    pub fn rpm_fraction(&self) -> f32 {
        ramp(self.rpm(), self.engine.idle, self.engine.red_line)
    }
}
