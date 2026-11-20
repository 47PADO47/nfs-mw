//! The six scenarios of docs/specs/ac-handling/scenarios.md and the scalar each one reads from a run of the planar car.

use ac_handling::mapping::{CarParams, CarSpec};

use crate::planar::{Planar, Sample};
use crate::script::{Run, Script, lead_time, run};

const G: f64 = 9.81;
/// 100 km/h in m/s, the spec's rounded figure.
pub const V100: f64 = 27.78;

/// The car of a scenario: its spec and tyres, the tick and the substep setting.
#[derive(Clone)]
pub struct Rig {
    /// The car.
    pub spec: CarSpec,
    /// Its tyres.
    pub params: CarParams,
    /// The tick, s.
    pub dt: f64,
}

impl Rig {
    /// A fresh car at rest.
    pub fn car(&self) -> Planar {
        Planar::new(self.spec.clone(), self.params.clone(), self.dt, 0)
    }

    /// `T_LEAD(v)` of this rig, rounded to 0.01 s.
    pub fn lead(&self, v: f64) -> Option<f64> {
        lead_time(|| self.car(), self.dt, v)
    }
}

/// The mean of `f` over a slice of samples.
fn mean(samples: &[Sample], f: impl Fn(&Sample) -> f64) -> f64 {
    samples.iter().map(f).sum::<f64>() / samples.len() as f64
}

/// S1: the largest mean of `|ay| / g` over any 0.5 s window in which `|beta| < 25 deg` throughout.
pub fn skidpad_g(rig: &Rig) -> f64 {
    let mut text = String::from("3:throttle=0.6;");
    for k in 0..30 {
        text.push_str(&format!("1:throttle={:.2},steer=0.12;", 0.12 + 0.02 * f64::from(k)));
    }
    let run = run(&mut rig.car(), &Script::parse(&text), rig.dt);
    let window = (0.5 / rig.dt).round() as usize;
    let limit = 25.0_f64.to_radians();
    let mut best = 0.0_f64;
    for slice in run.samples.windows(window) {
        if slice.iter().any(|sample| sample.beta().abs() >= limit) {
            continue;
        }
        best = best.max(mean(slice, |sample| sample.ay.abs()) / G);
    }
    best
}

/// What S2 reads.
#[derive(Debug, Clone, Copy)]
pub struct Braking {
    /// The distance from the first braking tick to a stop, normalised to 100 km/h, m.
    pub distance_m: f64,
    /// The raw distance, m.
    pub raw_m: f64,
    /// The speed at the first braking tick, m/s.
    pub v0: f64,
    /// The largest deceleration, g.
    pub peak_g: f64,
}

/// S2: braking from 100 km/h. `None` when the car cannot reach the speed.
pub fn braking(rig: &Rig) -> Option<Braking> {
    let lead = rig.lead(V100)?;
    let script = Script::parse(&format!("{lead}:throttle=1;6:brake=1"));
    let run = run(&mut rig.car(), &script, rig.dt);
    let first = run.starts[1];
    let before = run.samples[first - 1];
    let stop = run.samples[first..].iter().find(|sample| sample.speed() < 0.2)?;
    let raw_m = stop.dist - before.dist;
    let v0 = before.speed();
    let peak_g = run.samples[first..].iter().map(|sample| -sample.ax).fold(0.0_f64, f64::max) / G;
    Some(Braking { distance_m: raw_m * (V100 / v0).powi(2), raw_m, v0, peak_g })
}

/// S3: the understeer gradient in degrees per g, from the staircase of steering.
pub fn understeer(rig: &Rig) -> f64 {
    understeer_with(rig, 0.09).0
}

/// S3 with the hold throttle given; also the speeds of the points that count.
pub fn understeer_with(rig: &Rig, hold: f64) -> (f64, Vec<f64>) {
    let mut text = String::from("4:throttle=0.6;");
    for k in 1..=15 {
        text.push_str(&format!("2.5:throttle={hold},steer={:.2};", 0.01 * f64::from(k)));
    }
    let run = run(&mut rig.car(), &Script::parse(&text), rig.dt);
    let window = (0.5 / rig.dt).round() as usize;
    let wheelbase = rig.spec.wheelbase_m;
    let (mut xs, mut ys, mut speeds) = (Vec::new(), Vec::new(), Vec::new());
    for segment in 1..run.starts.len() {
        let end = run.starts.get(segment + 1).copied().unwrap_or(run.samples.len());
        let tail = &run.samples[end - window..end];
        let (v, ay) = (mean(tail, Sample::speed), mean(tail, |sample| sample.ay));
        let (delta, r) = (mean(tail, |sample| sample.delta), mean(tail, |sample| sample.r));
        let x = ay / G;
        if !(0.1..=0.5).contains(&x) {
            continue;
        }
        xs.push(x);
        ys.push((delta - wheelbase * r / v).to_degrees());
        speeds.push(v);
    }
    (slope(&xs, &ys), speeds)
}

/// The least-squares slope of `y` against `x`; 0 for fewer than two points.
fn slope(xs: &[f64], ys: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let covariance: f64 = xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let variance: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    covariance / variance
}

/// What S4 reads.
#[derive(Debug, Clone, Copy)]
pub struct Step {
    /// `(r_max - r_ss) / r_ss` in per cent.
    pub overshoot_pct: f64,
    /// The time from the step until the yaw rate stays within 5 per cent of steady, s.
    pub settle_s: f64,
    /// The steady yaw rate, rad/s.
    pub r_ss: f64,
    /// The steady lateral acceleration, g.
    pub ay_g: f64,
}

/// S4: step steer at 100 km/h, the planar car started at speed with rolling wheels.
pub fn step_steer(rig: &Rig, steer: f64) -> Step {
    let mut car = rig.car();
    car.start_rolling(V100);
    let script = Script::parse(&format!("1:throttle=0.12;4:throttle=0.12,steer={steer}"));
    let run = run(&mut car, &script, rig.dt);
    let step = run.starts[1];
    let last_second = (1.0 / rig.dt).round() as usize;
    let tail = &run.samples[run.samples.len() - last_second..];
    let r_ss = mean(tail, |sample| sample.r);
    let r_max = run.samples[step..].iter().map(|sample| sample.r).fold(f64::MIN, f64::max);
    let outside = run.samples[step..].iter().rposition(|sample| (sample.r - r_ss).abs() > 0.05 * r_ss.abs());
    Step {
        overshoot_pct: (r_max - r_ss) / r_ss * 100.0,
        settle_s: outside.map_or(0.0, |index| (index + 1) as f64 * rig.dt),
        r_ss,
        ay_g: mean(tail, |sample| sample.ay) / G,
    }
}

/// One rung of S5.
#[derive(Debug, Clone, Copy)]
pub struct Rung {
    /// The throttle of the rung.
    pub throttle: f64,
    /// The largest body slip angle of the run, degrees.
    pub beta_peak_deg: f64,
    /// The time from the end of the rung until the car has recovered for good, s; `None` if it never did.
    pub recovery_s: Option<f64>,
}

impl Rung {
    /// Whether the car spun: a slip angle of 45 degrees or more.
    pub fn spun(&self) -> bool {
        self.beta_peak_deg >= 45.0
    }
}

/// S5, one rung: a corner at 20 m/s, a power-on step to `throttle`, then the throttle lifted and the wheel centred.
pub fn rung(rig: &Rig, throttle: f64) -> Rung {
    let mut car = rig.car();
    car.start_rolling(20.0);
    let script = Script::parse(&format!("1:throttle=0.15,steer=0.10;1:throttle={throttle},steer=0.10;6:throttle=0.15"));
    let Run { samples, starts, .. } = run(&mut car, &script, rig.dt);
    let beta_peak = samples.iter().map(|sample| sample.beta().abs()).fold(0.0_f64, f64::max);
    let end = starts[2];
    let calm = |sample: &Sample| sample.r.abs() < 0.05 && sample.beta().abs().to_degrees() < 2.0;
    // The first tick from which the car stays calm to the end of the run, counted from the end of the rung.
    let first_calm = match samples[end..].iter().rposition(|sample| !calm(sample)) {
        None => Some(end),
        Some(last_bad) => Some(end + last_bad + 1).filter(|index| *index < samples.len()),
    };
    Rung {
        throttle,
        beta_peak_deg: beta_peak.to_degrees(),
        recovery_s: first_calm.map(|index| samples[index].t - samples[end].t),
    }
}

/// What S5 reads from its six rungs.
#[derive(Debug, Clone)]
pub struct Slide {
    /// The rungs, in order.
    pub rungs: Vec<Rung>,
    /// The lowest throttle whose slip angle reaches 5 degrees.
    pub breakaway: Option<f64>,
    /// The recovery of the highest rung that did not spin; `None` when every rung spun.
    pub recover_s: Option<f64>,
}

/// S5 over the rungs 0.5 to 1.0.
pub fn slide(rig: &Rig) -> Slide {
    let rungs: Vec<Rung> = [0.5, 0.6, 0.7, 0.8, 0.9, 1.0].into_iter().map(|throttle| rung(rig, throttle)).collect();
    let breakaway = rungs.iter().find(|rung| rung.beta_peak_deg >= 5.0).map(|rung| rung.throttle);
    let recover_s = rungs.iter().rev().find(|rung| !rung.spun()).and_then(|rung| rung.recovery_s);
    Slide { rungs, breakaway, recover_s }
}

/// What S6 reads.
#[derive(Debug, Clone, Copy)]
pub struct Launch {
    /// The distance after 3.0 s, m.
    pub d3_m: f64,
    /// The first time the speed is at least 100 km/h, s; `None` within the 6 s.
    pub t100_s: Option<f64>,
}

/// S6: full throttle from rest for 6 s.
pub fn launch(rig: &Rig) -> Launch {
    let run = run(&mut rig.car(), &Script::parse("6:throttle=1"), rig.dt);
    let at_three = (3.0 / rig.dt).round() as usize;
    let t100_s = run.samples.iter().find(|sample| sample.speed() >= V100).map(|sample| sample.t);
    Launch { d3_m: run.samples[at_three - 1].dist, t100_s }
}
