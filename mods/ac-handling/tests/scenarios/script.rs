//! `--drive-script` segments (docs/modding/drive-script.md): `seconds:key=value,key=value`, joined by `;`. Only the keys the
//! scenarios use (`throttle`, `brake`, `steer`) are read; an unspecified key is 0.

use crate::planar::{Pedals, Planar, Sample};

/// One segment: hold these pedals for this long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Length, s.
    pub secs: f64,
    /// The pedals and the wheel.
    pub pedals: Pedals,
}

/// A parsed script.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Script(pub Vec<Segment>);

impl Script {
    /// Parse the text of a script. Panics on a malformed one: the scripts are written in the tests.
    pub fn parse(text: &str) -> Script {
        let mut segments = Vec::new();
        for part in text.split(';').map(str::trim).filter(|part| !part.is_empty()) {
            let (secs, keys) = part.split_once(':').unwrap_or((part, ""));
            let mut pedals = Pedals::default();
            for pair in keys.split(',').map(str::trim).filter(|pair| !pair.is_empty()) {
                let (key, value) = pair.split_once('=').unwrap_or((pair, "1"));
                let value: f64 = value.parse().expect("a number");
                match key {
                    "throttle" => pedals.throttle = value,
                    "brake" => pedals.brake = value,
                    "steer" => pedals.steer = value,
                    other => panic!("the planar car does not read the key `{other}`"),
                }
            }
            segments.push(Segment { secs: secs.parse().expect("seconds"), pedals });
        }
        Script(segments)
    }

    /// The pedals of every tick of the script at tick length `dt`, with the index of each segment's first tick.
    pub fn ticks(&self, dt: f64) -> (Vec<Pedals>, Vec<usize>) {
        let mut pedals = Vec::new();
        let mut starts = Vec::new();
        for segment in &self.0 {
            starts.push(pedals.len());
            let count = (segment.secs / dt).round() as usize;
            pedals.extend(std::iter::repeat_n(segment.pedals, count));
        }
        (pedals, starts)
    }
}

/// The recording of one run: a sample per tick, and where each segment began.
pub struct Run {
    /// One sample per tick.
    pub samples: Vec<Sample>,
    /// The index of the first tick of each segment.
    pub starts: Vec<usize>,
}

/// Run `script` on `car`, recording every tick.
pub fn run(car: &mut Planar, script: &Script, dt: f64) -> Run {
    let (pedals, starts) = script.ticks(dt);
    let samples = pedals.iter().map(|tick| car.step(tick)).collect();
    Run { samples, starts }
}

/// The time a full-throttle run from rest takes to reach `v` m/s (`T_LEAD`, rounded to 0.01 s): the end time of the first tick
/// whose speed is at least `v` (the convention that gives the spec's 4.98 s and 3.57 s).
pub fn lead_time(make: impl Fn() -> Planar, dt: f64, v: f64) -> Option<f64> {
    let mut car = make();
    let full = Pedals { throttle: 1.0, ..Pedals::default() };
    for _ in 0..((30.0 / dt) as usize) {
        let sample = car.step(&full);
        if sample.speed() >= v {
            return Some(((sample.t + dt) * 100.0).round() / 100.0);
        }
    }
    None
}
