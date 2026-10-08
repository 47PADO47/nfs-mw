use std::f32::consts::{PI, TAU};

/// Left and right front wheel angles in radians (+ = turning right).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WheelAngles {
    pub left: f32,
    pub right: f32,
}

/// Splits the inside-wheel target angle `target_deg` (+ = right) into the two front wheel angles so the
/// outside wheel turns less (the small-angle form of `cot(out) = cot(in) + track / wheelbase`).
pub fn ackermann(target_deg: f32, wheel_base: f32, track_width: f32) -> WheelAngles {
    let turns = target_deg / 360.0;
    let mut inside = turns.abs() * TAU;
    if inside > PI {
        inside -= TAU;
    }
    let denom = track_width * inside + wheel_base;
    let outside = if denom.abs() > 1e-6 { wheel_base * inside / denom } else { inside };
    if turns >= 0.0 {
        WheelAngles { left: outside, right: inside }
    } else {
        WheelAngles { left: -inside, right: -outside }
    }
}
