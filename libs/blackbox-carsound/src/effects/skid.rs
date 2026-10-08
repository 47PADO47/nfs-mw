//! Tire skids: one loop per axle. Spec §7 and the decisions of §10.

use super::{LoopId, SoundCommand, SoundRef};
use crate::input::{CarInput, WheelInput};
use crate::math::{ramp, slew};
use crate::tuning::SkidTuning;

/// Smoothing per tick of the sideways component and of the load (spec: 500 and 3000 of 32767).
const SIDE_STEP: f32 = 500.0 / 32767.0;
const LOAD_STEP: f32 = 3000.0 / 32767.0;
/// How much one component must exceed the other before the loop swaps.
const SWAP_MARGIN: f32 = 0.1;
/// The loop stops below this volume.
const AUDIBLE: f32 = 0.01;

/// One axle.
#[derive(Debug, Clone, Copy, Default)]
struct Axle {
    forward: f32,
    /// The sideways slip before smoothing.
    raw_side: f32,
    side: f32,
    load: f32,
    surface: u8,
    playing: bool,
    volume: f32,
    pitch: f32,
    sideways: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SkidFx {
    tuning: SkidTuning,
    axles: [Axle; 2],
    /// The mixer inputs: sideways slip and load (Q15), smoothed.
    side_signal: f32,
    load_signal: f32,
}

/// A full-scale slip as the mixer input: 1023 shifted left by 5.
const SIGNAL_FULL: f32 = 1023.0 * 32.0;

/// The normalised forward and sideways slip and the load of one wheel, each 0 to 1.
fn wheel_levels(w: &WheelInput) -> (f32, f32, f32) {
    let dead = 0.2 * w.tolerated_slip.abs();
    let slip = if w.slip.abs() <= dead { 0.0 } else { w.slip - dead * w.slip.signum() };
    let forward = (slip * 200.0).clamp(-1023.0, 1023.0).abs() / 1023.0;
    let side = (-w.skid * 81.0).clamp(-1023.0, 1023.0).abs() / 1023.0;
    (forward, side, ramp(w.load, 4000.0, 10000.0, 0.0, 1.0))
}

impl SkidFx {
    pub fn new(tuning: SkidTuning) -> Self {
        Self { tuning, axles: [Axle::default(); 2], side_signal: 0.0, load_signal: 0.0 }
    }

    pub fn tick(&mut self, input: &CarInput) {
        // Wheel order: front left, front right, rear right, rear left.
        for (axle, pair) in self.axles.iter_mut().zip([[0usize, 1], [2, 3]]) {
            let wheels = pair.map(|i| input.wheels[i]);
            let grounded: Vec<&WheelInput> = wheels.iter().filter(|w| w.on_ground).collect();
            let n = grounded.len().max(1) as f32;
            let sum = |f: fn(&WheelInput) -> (f32, f32, f32)| {
                grounded.iter().map(|w| f(w)).fold((0.0, 0.0, 0.0), |a, v| (a.0 + v.0, a.1 + v.1, a.2 + v.2))
            };
            let (forward, side, load) = sum(wheel_levels);
            let (forward, side, load) = (forward / n, side / n, load / n);
            axle.forward = forward;
            axle.raw_side = side;
            axle.side = slew(axle.side, side, SIDE_STEP);
            axle.load = slew(axle.load, load.max(forward).max(side), LOAD_STEP);
            let skid_surface = |w: &&WheelInput| {
                if w.blown { self.tuning.blown_tire_surface } else { w.skid_surface }
            };
            axle.surface = grounded.iter().map(skid_surface).max().unwrap_or(0);
            let strongest = axle.forward.max(axle.side);
            // The squeal and the burnout loop swap only when the other clearly takes over, so a wheel between
            // them does not restart the loop every frame.
            axle.sideways = if axle.sideways {
                axle.side + SWAP_MARGIN >= axle.forward
            } else {
                axle.side > axle.forward + SWAP_MARGIN
            };
            axle.volume = if grounded.is_empty() { 0.0 } else { strongest * (0.3 + 0.7 * load) };
            axle.pitch = 0.9 + 0.2 * strongest;
        }
        let forward = self.axles.iter().map(|a| a.forward).fold(0.0, f32::max) * SIGNAL_FULL;
        let side = self.axles.iter().map(|a| a.raw_side).fold(0.0, f32::max) * SIGNAL_FULL;
        self.side_signal = slew(self.side_signal, side, 500.0);
        self.load_signal = slew(self.load_signal, forward.max(side), 3000.0);
    }

    /// `(forward, sideways, load)` as the mixer inputs, Q15.
    pub fn signals(&self) -> (f32, f32, f32) {
        let forward = self.axles.iter().map(|a| a.forward).fold(0.0, f32::max) * SIGNAL_FULL;
        (forward, self.side_signal, self.load_signal)
    }

    pub fn emit(&mut self, out: &mut Vec<SoundCommand>) {
        for (i, axle) in self.axles.iter_mut().enumerate() {
            let id = LoopId::Skid(i as u8);
            if axle.volume < AUDIBLE {
                if std::mem::take(&mut axle.playing) {
                    out.push(SoundCommand::StopLoop(id));
                }
                continue;
            }
            axle.playing = true;
            let sound = SoundRef::Skid { surface: axle.surface, sideways: axle.sideways };
            out.push(SoundCommand::SetLoop { id, sound, volume: axle.volume, pitch: axle.pitch });
        }
    }
}
