//! Traction control, ABS, stability control and brake bias (docs/specs/ac-handling/aids.md section 4). Pure: one call is one tick of
//! the controls hook, in the order traction control, brake bias, ABS, stability control, clamp.

use std::f64::consts::TAU;

/// The check period of traction control and ABS, s.
pub const T_CHK: f64 = 0.05;
/// No traction control below this speed, m/s.
pub const V_TC: f64 = 1.0;
/// No ABS below this speed (20 km/h), m/s.
pub const V_ABS: f64 = 5.556;
/// No stability control below this speed (5 km/h), m/s.
pub const V_ESC: f64 = 1.389;
/// Slip ratio limits of traction control, levels 1 to 5.
const KAPPA_TC: [f64; 5] = [0.20, 0.15, 0.12, 0.09, 0.06];
/// Throttle share removed by traction control, levels 1 to 5.
const A_TC: [f64; 5] = [0.50, 0.65, 0.80, 0.90, 1.00];
/// Lock limits of ABS, levels 1 to 5.
const KAPPA_ABS: [f64; 5] = [0.30, 0.25, 0.20, 0.15, 0.10];
/// Brake share removed by ABS, levels 1 to 5.
const A_ABS: [f64; 5] = [0.70, 0.80, 0.90, 1.00, 1.00];
/// Stability control gain, levels 0 to 4.
const GAIN_ESC: [f64; 5] = [0.0, 0.25, 0.50, 0.75, 1.00];
const K_ESC: f64 = 30.0;
const TAU_ESC: f64 = 0.04;
const MU_REF: f64 = 1.0;
const BETA_MIN: f64 = 0.30;
const BETA_MAX: f64 = 0.80;
const BIAS_STEP: f64 = 0.005;
/// The most the host accepts as a brake demand.
pub const BRAKE_MAX: f64 = 1.5;
/// The highest level of traction control and ABS.
pub const MAX_LEVEL: u32 = 5;
/// The highest level of stability control.
pub const MAX_ESC_LEVEL: u32 = 4;
/// The most clicks of brake bias either way.
pub const MAX_CLICKS: i32 = 20;

/// The levels the player set: 0 is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Levels {
    /// Traction control, 0 to 5.
    pub tc: u32,
    /// ABS, 0 to 5.
    pub abs: u32,
    /// Stability control, 0 to 4.
    pub esc: u32,
    /// Brake bias in clicks of 0.005 of the front share, -20 to 20.
    pub bias_clicks: i32,
}

impl Levels {
    /// The same levels inside their ranges.
    pub fn clamped(self) -> Levels {
        Levels {
            tc: self.tc.min(MAX_LEVEL),
            abs: self.abs.min(MAX_LEVEL),
            esc: self.esc.min(MAX_ESC_LEVEL),
            bias_clicks: self.bias_clicks.clamp(-MAX_CLICKS, MAX_CLICKS),
        }
    }
}

/// The constants of the vehicle (read from the spec when its revision changes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AidCar {
    /// Mass, kg.
    pub mass: f64,
    /// Front track, m.
    pub track_f: f64,
    /// Static wheel loads, N.
    pub w_static: [f64; 4],
    /// The front share of the host's maximum brake torque.
    pub beta_ref: f64,
    /// The front share the player's bias starts from (the host's own split by default).
    pub beta_base: f64,
    /// Which wheels receive drive torque.
    pub driven: [bool; 4],
}

/// What one tick tells the aids (the query, converted).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AidIn {
    /// The tick, s.
    pub dt: f64,
    /// The shaped throttle.
    pub throttle: f64,
    /// The shaped brake.
    pub brake: f64,
    /// Speed along the heading, m/s.
    pub v: f64,
    /// Slip ratio per wheel, last tick.
    pub kappa: [f64; 4],
    /// Slip angle per wheel in radians, last tick.
    pub alpha: [f64; 4],
    /// Which wheels pressed on the ground.
    pub loaded: [bool; 4],
}

/// What the aids answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AidOut {
    /// The throttle the engine is given.
    pub throttle: f64,
    /// The brake demand per wheel, 0 to 1.5.
    pub brake: [f64; 4],
    /// Traction control is cutting.
    pub tc_active: bool,
    /// ABS is releasing each wheel.
    pub abs_active: [bool; 4],
    /// Stability control is braking.
    pub esc_active: bool,
}

/// The state of the aids (section 3).
#[derive(Debug, Clone, PartialEq)]
pub struct AidState {
    /// Time since the last traction control check, s.
    pub tc_acc: f64,
    /// Traction control is cutting.
    pub tc_cut: bool,
    /// Time since the last ABS check, s.
    pub abs_acc: f64,
    /// ABS releases each wheel.
    pub abs_rel: [bool; 4],
    /// The stability brake demand, 0 to 1.
    pub esc_y: f64,
    /// The front wheel stability control brakes: 0 left, 1 right.
    pub esc_wheel: usize,
    levels: Levels,
}

impl AidState {
    /// A state with nothing active.
    pub fn new() -> AidState {
        AidState {
            tc_acc: 0.0,
            tc_cut: false,
            abs_acc: 0.0,
            abs_rel: [false; 4],
            esc_y: 0.0,
            esc_wheel: 0,
            levels: Levels::default(),
        }
    }

    /// Reset what a change of a level resets (section 3).
    fn follow(&mut self, levels: Levels) {
        if levels.tc != self.levels.tc {
            self.tc_acc = 0.0;
            self.tc_cut = false;
        }
        if levels.abs != self.levels.abs {
            self.abs_acc = 0.0;
            self.abs_rel = [false; 4];
        }
        if levels.esc != self.levels.esc && levels.esc == 0 {
            self.esc_y = 0.0;
            self.esc_wheel = 0;
        }
        self.levels = levels;
    }
}

impl Default for AidState {
    fn default() -> Self {
        AidState::new()
    }
}

/// Traction control (4.1): the throttle after the cut.
fn traction(car: &AidCar, level: u32, st: &mut AidState, inp: &AidIn) -> f64 {
    if level == 0 || inp.v.abs() < V_TC {
        st.tc_cut = false;
        st.tc_acc = 0.0;
        return inp.throttle;
    }
    st.tc_acc += inp.dt;
    if st.tc_acc + 1.0e-6 >= T_CHK {
        st.tc_acc = 0.0;
        let slip =
            (0..4).filter(|i| car.driven[*i] && inp.loaded[*i]).map(|i| inp.kappa[i]).fold(f64::NEG_INFINITY, f64::max);
        st.tc_cut = slip > KAPPA_TC[level as usize - 1];
    }
    let cut = if st.tc_cut { A_TC[level as usize - 1] } else { 0.0 };
    inp.throttle * (1.0 - cut)
}

/// Brake bias (4.2): the brake of each wheel before ABS and stability control.
fn bias(car: &AidCar, clicks: i32, brake: f64) -> [f64; 4] {
    let beta = (car.beta_base + BIAS_STEP * f64::from(clicks)).clamp(BETA_MIN, BETA_MAX);
    let front = brake * beta / car.beta_ref;
    let rear = brake * (1.0 - beta) / (1.0 - car.beta_ref);
    [front, front, rear, rear]
}

/// ABS (4.3), four channels.
fn anti_lock(level: u32, st: &mut AidState, inp: &AidIn, b: &mut [f64; 4]) {
    if level == 0 || inp.v.abs() < V_ABS {
        st.abs_acc = 0.0;
        st.abs_rel = [false; 4];
        return;
    }
    st.abs_acc += inp.dt;
    if st.abs_acc + 1.0e-6 >= T_CHK {
        st.abs_acc = 0.0;
        for i in 0..4 {
            st.abs_rel[i] = inp.loaded[i] && inp.kappa[i] < -KAPPA_ABS[level as usize - 1];
        }
    }
    for i in 0..4 {
        let release = if st.abs_rel[i] { A_ABS[level as usize - 1] } else { 0.0 };
        b[i] *= 1.0 - release;
    }
}

/// The stability target of 4.4: 0 to 1, and the wheel it brakes when it acts.
fn esc_target(car: &AidCar, level: u32, st: &mut AidState, inp: &AidIn) -> f64 {
    if level == 0 || inp.v.abs() < V_ESC {
        return 0.0;
    }
    let a_front = 0.5 * (inp.alpha[0] + inp.alpha[1]);
    let a_rear = 0.5 * (inp.alpha[2] + inp.alpha[3]);
    let delta = a_rear.abs() - a_front.abs();
    if delta <= 0.0 {
        return 0.0;
    }
    let moment = car.mass * delta * GAIN_ESC[level as usize] * K_ESC;
    st.esc_wheel = usize::from(a_rear > 0.0);
    let force = moment / (0.5 * car.track_f);
    (force / (MU_REF * car.w_static[st.esc_wheel])).min(1.0)
}

/// One tick of the aids.
pub fn aids_step(car: &AidCar, levels: Levels, st: &mut AidState, inp: &AidIn) -> AidOut {
    let levels = levels.clamped();
    st.follow(levels);
    let throttle = traction(car, levels.tc, st, inp);
    let mut b = bias(car, levels.bias_clicks, inp.brake);
    anti_lock(levels.abs, st, inp, &mut b);
    let target = esc_target(car, levels.esc, st, inp);
    st.esc_y += (target - st.esc_y) * (1.0 - (-inp.dt / TAU_ESC).exp());
    if st.esc_y > 0.0 && levels.esc > 0 && !st.abs_rel[st.esc_wheel] {
        b[st.esc_wheel] = b[st.esc_wheel].max(st.esc_y);
    }
    AidOut {
        throttle: throttle.clamp(0.0, 1.0),
        brake: b.map(|value| value.clamp(0.0, BRAKE_MAX)),
        tc_active: st.tc_cut,
        abs_active: st.abs_rel,
        esc_active: st.esc_y > 0.02,
    }
}

/// A slip angle from the query's turns to radians.
pub fn turns_to_rad(turns: f64) -> f64 {
    turns * TAU
}

impl AidCar {
    /// The constants of a car from its spec: the mass, the front track, the static wheel loads, the front share of the brake
    /// torque and the driven wheels. `None` when a number the aids divide by is not usable.
    pub fn from_spec(spec: &crate::mapping::CarSpec) -> Option<AidCar> {
        let [front, rear] = spec.static_corner_loads();
        let torque = spec.brake_torque_nm;
        let share = torque[0] / (torque[0] + torque[1]);
        let usable = spec.mass_kg > 0.0 && spec.track_m[0] > 0.0 && front > 0.0 && rear > 0.0 && share.is_finite();
        if !usable {
            return None;
        }
        let beta_ref = share.clamp(0.05, 0.95);
        let driven = match spec.drive {
            crate::mapping::Drive::Front => [true, true, false, false],
            crate::mapping::Drive::Rear => [false, false, true, true],
            crate::mapping::Drive::All => [true; 4],
        };
        Some(AidCar {
            mass: spec.mass_kg,
            track_f: spec.track_m[0],
            w_static: [front, front, rear, rear],
            beta_ref,
            beta_base: beta_ref,
            driven,
        })
    }
}
