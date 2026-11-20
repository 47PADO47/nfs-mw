//! The planar reference car of the appendix of docs/specs/ac-handling/scenarios.md: a rigid body with three degrees of freedom
//! (forward and lateral velocity, yaw rate), no suspension, explicit Euler at the tick, driven through the model's own
//! `wheel_tick` for each of the four wheels. It exists to check the model against its targets without a game.

use ac_handling::mapping::{CarParams, CarSpec};
use ac_handling::wheel::{WheelInput, WheelState, wheel_tick};

const G: f64 = 9.81;
/// Air density of the appendix, kg/m^3.
const RHO: f64 = 1.2;
/// Steering lock of the planar car: the wheel angle at `steer = 1`, rad.
const STEER_LOCK: f64 = 0.5;
/// The rate limit of the wheel angle, rad/s.
const STEER_RATE: f64 = 4.0;
/// The host's wheel inertia, kg m^2.
const HOST_INERTIA: f64 = 10.0;

/// The pedals and the wheel of one tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pedals {
    /// 0 to 1.
    pub throttle: f64,
    /// 0 to 1.
    pub brake: f64,
    /// -1 to 1, positive to the right.
    pub steer: f64,
}

/// What the harness records after every tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Time at the start of the tick, s.
    pub t: f64,
    /// Forward velocity of the body, m/s.
    pub vf: f64,
    /// Lateral velocity, m/s, positive to the right.
    pub vr: f64,
    /// Yaw rate, rad/s, positive turning right.
    pub r: f64,
    /// Specific lateral force, m/s^2.
    pub ay: f64,
    /// Specific longitudinal force, m/s^2.
    pub ax: f64,
    /// Front wheel angle, rad.
    pub delta: f64,
    /// Distance along the body axis since the start, m.
    pub dist: f64,
    /// What the pedals were this tick.
    pub pedals: Pedals,
}

impl Sample {
    /// The speed of the body.
    pub fn speed(&self) -> f64 {
        self.vf.hypot(self.vr)
    }

    /// The body slip angle, rad.
    pub fn beta(&self) -> f64 {
        self.vr.atan2(self.vf.abs().max(0.5))
    }
}

/// The car and its tyres.
pub struct Planar {
    spec: CarSpec,
    params: CarParams,
    wheels: [WheelState; 4],
    spin: [f64; 4],
    vf: f64,
    vr: f64,
    r: f64,
    delta: f64,
    ax: f64,
    ay: f64,
    dist: f64,
    time: f64,
    dt: f64,
    substeps: u32,
}

impl Planar {
    /// A car at rest, with the tick `dt` and the number of substeps (0 automatic).
    pub fn new(spec: CarSpec, params: CarParams, dt: f64, substeps: u32) -> Planar {
        Planar {
            spec,
            params,
            wheels: [WheelState::fresh(); 4],
            spin: [0.0; 4],
            vf: 0.0,
            vr: 0.0,
            r: 0.0,
            delta: 0.0,
            ax: 0.0,
            ay: 0.0,
            dist: 0.0,
            time: 0.0,
            dt,
            substeps,
        }
    }

    /// Put the car at speed `v` m/s with the wheels rolling (the start of S4 and S5).
    pub fn start_rolling(&mut self, v: f64) {
        self.vf = v;
        for (index, spin) in self.spin.iter_mut().enumerate() {
            *spin = v / self.spec.tyre_radius_m[usize::from(index >= 2)];
        }
    }

    /// Torque at the rear axle for `throttle`: the gear with the largest wheel torque below 0.97 of the redline, the engine
    /// torque from the power curve, times the ratios and 0.9 (the appendix).
    fn axle_torque(&self, throttle: f64) -> f64 {
        let s = &self.spec;
        let mean_spin = (self.spin[2] + self.spin[3]) / 2.0;
        let w_peak = s.max_power_rpm * std::f64::consts::TAU / 60.0;
        let mut best = 0.0_f64;
        for ratio in &s.gear_ratios {
            let rpm = mean_spin * ratio * s.final_drive * 60.0 / std::f64::consts::TAU;
            if rpm >= 0.97 * s.redline_rpm {
                continue;
            }
            let w = rpm.max(3500.0) * std::f64::consts::TAU / 60.0;
            let torque = (1.05 * s.max_power_w / w_peak).min(s.max_power_w / w);
            best = best.max(torque * ratio * s.final_drive * 0.9);
        }
        throttle * best
    }

    /// One tick of `dt` with the given pedals.
    pub fn step(&mut self, pedals: &Pedals) -> Sample {
        let s = self.spec.clone();
        let dt = self.dt;
        let (m, l, f) = (s.mass_kg, s.wheelbase_m, s.front_weight_fraction);
        let (a, b) = ((1.0 - f) * l, f * l);
        let iz = 1.05 * m * a * b;
        // the front wheels follow the steering with a rate limit
        let target = STEER_LOCK * pedals.steer;
        let move_by = (target - self.delta).clamp(-STEER_RATE * dt, STEER_RATE * dt);
        self.delta += move_by;
        let delta = self.delta;
        // loads from the last tick's accelerations
        let (ax, ay, h) = (self.ax, self.ay, s.cg_height_m);
        let front = f * m * G / 2.0 - m * ax * h / (2.0 * l);
        let rear = (1.0 - f) * m * G / 2.0 + m * ax * h / (2.0 * l);
        let lat_f = 0.5 * m * ay * h / s.track_m[0];
        let lat_r = 0.5 * m * ay * h / s.track_m[1];
        let loads = [front + lat_f, front - lat_f, rear + lat_r, rear - lat_r].map(|load| load.max(0.0));
        let xs = [-s.track_m[0] / 2.0, s.track_m[0] / 2.0, -s.track_m[1] / 2.0, s.track_m[1] / 2.0];
        let zs = [a, a, -b, -b];
        let axle_torque = self.axle_torque(pedals.throttle);
        let (mut sum_f, mut sum_r, mut moment) = (0.0, 0.0, 0.0);
        for i in 0..4 {
            let rear_axle = usize::from(i >= 2);
            let d = if i < 2 { delta } else { 0.0 };
            let v_f = self.vf - self.r * xs[i];
            let v_r = self.vr + self.r * zs[i];
            let input = WheelInput {
                radius: s.tyre_radius_m[rear_axle],
                inertia: HOST_INERTIA,
                fz: loads[i],
                vx: v_f * d.cos() + v_r * d.sin(),
                vy: -v_f * d.sin() + v_r * d.cos(),
                spin: self.spin[i],
                drive_torque: if i >= 2 { axle_torque / 2.0 } else { 0.0 },
                brake_torque: pedals.brake * s.brake_torque_nm[rear_axle],
                dt,
                corner_mass: if i < 2 { f * m / 2.0 } else { (1.0 - f) * m / 2.0 },
                surf_lat: 1.0,
                surf_drive: 1.0,
                surf_rolling: 1.0,
                grip: 1.0,
                loaded: loads[i] > 0.0,
            };
            let out = wheel_tick(self.params.axle(i), &mut self.wheels[i], &input, self.substeps)
                .expect("the planar car's wheels are always valid");
            self.spin[i] = out.spin;
            let force_f = out.fx * d.cos() - out.fy * d.sin();
            let force_r = out.fx * d.sin() + out.fy * d.cos();
            sum_f += force_f;
            sum_r += force_r;
            moment += zs[i] * force_r - xs[i] * force_f;
        }
        let speed = self.vf.hypot(self.vr);
        sum_f -= 0.5 * RHO * s.drag_area_m2 * speed * self.vf;
        sum_r -= 0.5 * RHO * s.drag_area_m2 * speed * self.vr;
        let (vf, vr, r) = (self.vf, self.vr, self.r);
        let t = self.time;
        self.vf += dt * (sum_f / m + r * vr);
        // The distance advances with the velocity of the end of the tick: this is the convention that reproduces the spec's
        // distances (see NOTES.md, spec questions).
        self.dist += self.vf * dt;
        self.vr += dt * (sum_r / m - r * vf);
        self.r += dt * moment / iz;
        self.ax = sum_f / m;
        self.ay = sum_r / m;
        self.time += dt;
        Sample {
            t,
            vf: self.vf,
            vr: self.vr,
            r: self.r,
            ay: self.ay,
            ax: self.ax,
            delta,
            dist: self.dist,
            pedals: *pedals,
        }
    }
}
