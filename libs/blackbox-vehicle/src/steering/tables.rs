//! Lookup tables of the steering model. The speed axis is the car-frame forward speed over 0..160 (m/s;
//! see the README for the open unit question).

/// Largest steering angle of the inside front wheel, degrees.
pub const ABSOLUTE_MAX_STEERING: f32 = 45.0;
/// Upper end of the speed axis of the range tables.
pub const MAX_SPEED_AXIS: f32 = 160.0;

/// Maximum angle (deg) against speed for a gamepad or keyboard.
pub const RANGE_PAD: [f32; 10] = [40.0, 20.0, 10.0, 5.5, 4.5, 3.25, 2.9, 2.9, 2.9, 2.9];
/// Maximum angle (deg) against speed for a racing wheel with speed-sensitive range.
pub const RANGE_WHEEL: [f32; 10] = [45.0, 15.0, 11.0, 8.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0];
/// Speed factor of both the braking widening and the steering rate.
pub const STEER_SPEED: [f32; 10] = [1.0, 1.0, 1.0, 0.56, 0.5, 0.35, 0.3, 0.3, 0.3, 0.3];
/// Range multiplier against the recent average input magnitude 0..1.
pub const RANGE_COEF: [f32; 6] = [1.0, 1.05, 1.1, 1.2, 1.3, 1.4];
/// Rate multiplier against the input speed 0..10 per second.
pub const INPUT_SPEED_COEF: [f32; 6] = [1.0, 1.05, 1.1, 1.5, 2.2, 3.1];
/// Rate multiplier against the recent average input magnitude 0..1.
pub const INPUT_COEF: [f32; 6] = [1.0, 1.05, 1.1, 1.2, 1.3, 1.4];
/// Post-collision blend against seconds since the hit: `(seconds, blend)`.
pub const COLLISION_BLEND: [(f32, f32); 4] = [(0.0, 0.2), (0.2, 0.5), (0.5, 0.7), (0.7, 1.0)];
/// Input remap over -1..1 (21 points); the first 11 entries, the rest mirror them.
const REMAP_HALF: [f32; 11] = [-1.0, -0.736, -0.542, -0.4, -0.292, -0.214, -0.16, -0.123, -0.078, -0.036, 0.0];

/// The 21-point remap that flattens small stick deflections (the "medium" set).
pub fn input_remap() -> [f32; 21] {
    let mut t = [0.0; 21];
    for (i, v) in REMAP_HALF.iter().enumerate() {
        t[i] = *v;
        t[20 - i] = -*v;
    }
    t
}
