//! Road noise (a loop per side of the car) and wind noise. Spec §9.
//!
//! The mixer maps that scale these in the original are not specified; the gains below are a decision
//! (`docs/specs/engine-sound-effects.md` §10).

use super::{LoopId, SoundCommand, SoundRef};
use crate::input::{CarInput, WheelInput};

/// Gain of the road noise and the wind noise relative to a full-scale loop.
const ROAD_GAIN: f32 = 0.35;
const WIND_GAIN: f32 = 0.4;
/// Below this speed (m/s) the wind is silent; the wind speed is clamped to 40 m/s.
const WIND_FLOOR: f32 = 2.0;
const WIND_TOP: f32 = 40.0;

/// `(speed mph, Q15 volume)` points of the road noise by speed.
const ROAD_GRAPH: [(f32, f32); 5] = [(0.0, 0.0), (60.0, 28000.0), (100.0, 32500.0), (150.0, 24500.0), (175.0, 18000.0)];

fn road_graph(mph: f32) -> f32 {
    let last = ROAD_GRAPH[ROAD_GRAPH.len() - 1];
    if mph >= last.0 {
        return last.1;
    }
    for pair in ROAD_GRAPH.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if mph <= x1 {
            return y0 + (mph - x0) / (x1 - x0) * (y1 - y0);
        }
    }
    0.0
}

/// One side of the car.
#[derive(Debug, Clone, Copy, Default)]
struct Side {
    playing: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RoadFx {
    sides: [Side; 2],
    /// Slip (forward, sideways) and traction use per side from the latest tick.
    slip: [f32; 2],
    traction: [f32; 2],
}

/// The wheels of a side: left is front left and rear left (indices 0 and 3), right the other two.
fn side_wheels(input: &CarInput, side: usize) -> [&WheelInput; 2] {
    match side {
        0 => [&input.wheels[0], &input.wheels[3]],
        _ => [&input.wheels[1], &input.wheels[2]],
    }
}

impl RoadFx {
    pub fn tick(&mut self, input: &CarInput) {
        for side in 0..2 {
            let wheels: Vec<&WheelInput> = side_wheels(input, side).into_iter().filter(|w| w.on_ground).collect();
            let forward: f32 = wheels.iter().map(|w| w.slip).sum();
            let lateral: f32 = wheels.iter().map(|w| w.skid).sum();
            self.slip[side] = forward.hypot(lateral);
            let usage: f32 = side_wheels(input, side).iter().map(|w| w.traction_usage.abs()).sum();
            self.traction[side] = usage / 2.0;
        }
    }

    pub fn emit(&mut self, input: &CarInput, out: &mut Vec<SoundCommand>) {
        let mph = input.speed_mph();
        let base = road_graph(mph);
        let pitch0 = 1500.0 + 3000.0 * (mph / 100.0).clamp(0.0, 1.0);
        for side in 0..2 {
            let id = LoopId::Road(side as u8);
            let grounded = side_wheels(input, side).into_iter().find(|w| w.on_ground);
            let loop_id = grounded.map_or(0, |w| w.road_noise_loop);
            let (slip, traction) = (self.slip[side], self.traction[side]);
            // Only the left side's volume has the traction term (kept from the original).
            let traction_gain = if side == 0 { 1.0 + (traction * 0.1).min(0.1) } else { 1.1 };
            let volume = (base * (1.0 + (slip * 0.01).min(0.15)) * traction_gain).min(32000.0) / 32767.0;
            let pitch = (pitch0 * (1.0 + (slip * 0.01).min(0.2)) * (1.0 + (traction * 0.15).min(0.15))).min(6000.0);
            if loop_id == 0 || volume <= 0.0 {
                if std::mem::take(&mut self.sides[side].playing) {
                    out.push(SoundCommand::StopLoop(id));
                }
                continue;
            }
            self.sides[side].playing = true;
            out.push(SoundCommand::SetLoop {
                id,
                sound: SoundRef::RoadNoise(loop_id),
                volume: volume * ROAD_GAIN,
                // The data's pitch is on a 4096 = 1 scale around the speed-dependent value; as a ratio it
                // runs from 0.75 to 1.5 over the speed range.
                pitch: (pitch / 3000.0).clamp(0.5, 2.0),
            });
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WindFx {
    playing: bool,
}

impl WindFx {
    pub fn emit(&mut self, input: &CarInput, out: &mut Vec<SoundCommand>) {
        let speed = input.speed.abs();
        if !speed.is_finite() || speed < WIND_FLOOR {
            if std::mem::take(&mut self.playing) {
                out.push(SoundCommand::StopLoop(LoopId::Wind));
            }
            return;
        }
        self.playing = true;
        let ratio = speed.min(WIND_TOP) / WIND_TOP;
        out.push(SoundCommand::SetLoop {
            id: LoopId::Wind,
            sound: SoundRef::Wind,
            volume: ratio * WIND_GAIN,
            pitch: 0.8 + 0.4 * ratio,
        });
    }
}
