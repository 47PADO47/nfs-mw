//! The rotor loop's class words and the pure mappings behind them. Spec: `docs/specs/ai-helicopter.md` section 7.
//! The clamps of the twelve words are the original's publisher (`[decomp]`). Word meanings are `[unconfirmed]` except
//! where noted. The map already does the distance rolloff and the rotor's pitch, so volume and pitch pass through.

/// The range word the bank is given, centimetres (25000). Same as the traffic modules' range word. [unconfirmed]
const RANGE_CM: i32 = 25000;
/// The publisher clamps the signed distance to plus or minus this. [decomp]
const DISTANCE_CLAMP: f32 = 100.0;
/// Forward speed, m/s, that reads as full rotor speed (spec section 7). [decomp]
const FULL_SPEED: f32 = 500.0;
/// Below this forward speed the speed word is the idle value (spec section 7). [decomp]
const IDLE_BELOW: f32 = 0.25;
const IDLE_SPEED: f32 = 0.5;

/// What the words are built from, for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotorInputs {
    /// The mixer's rotor volume, 0 to 32767 (the map's rolloff applied).
    pub volume: i32,
    /// The mixer's azimuth of the rotor from the listener, turns of 65536.
    pub azimuth: u16,
    /// The mixer's rotor pitch, Q12 (4096 is 1.0).
    pub pitch: i32,
    /// The helicopter is in front of the listener.
    pub in_front: bool,
    /// Distance from the listener, metres.
    pub distance: f32,
    /// Speed along the helicopter's heading, m/s (negative when reversing).
    pub forward_speed: f32,
    /// Angle between the helicopter's heading and the bearing to the listener, radians (0 to pi).
    pub bearing: f32,
}

/// The distance word: the metres, clamped to plus or minus 100, negative when the rotor is behind. [decomp]
pub fn distance_word(distance: f32, in_front: bool) -> i32 {
    let metres = distance.clamp(0.0, DISTANCE_CLAMP) as i32;
    if in_front {
        return metres;
    }
    -metres
}

/// The speed fraction: `min(1, forward / 500)`, or the idle 0.5 below 0.25 m/s. [decomp]
pub fn speed_fraction(forward_speed: f32) -> f32 {
    if !forward_speed.is_finite() {
        return 0.0;
    }
    if forward_speed < IDLE_BELOW {
        return IDLE_SPEED;
    }
    (forward_speed / FULL_SPEED).min(1.0)
}

/// The speed word, 0 to 1023. [decomp]
pub fn speed_word(forward_speed: f32) -> i32 {
    (speed_fraction(forward_speed) * 1023.0) as i32
}

/// The twelve class words of `FX_Helicopter`, in the order the publisher stores them (word 0 at byte +4 of the
/// original's class object). [decomp] for the clamps; the meaning of each word is in the comments.
pub fn class_words(inputs: RotorInputs) -> [i32; 12] {
    let rotation = (inputs.bearing.clamp(0.0, std::f32::consts::PI) / std::f32::consts::PI * 65535.0) as i32;
    [
        inputs.volume.clamp(0, 32767), // 0 volume, the map's output [unconfirmed]
        i32::from(inputs.azimuth as i16).clamp(-8192, 8192), // 1 azimuth, signed [unconfirmed]
        i32::from(inputs.in_front),    // 2 in front flag [unconfirmed]
        inputs.pitch.clamp(0, 32767),  // 3 pitch, Q12, the map's output [unconfirmed]
        rotation,                      // 4 bearing to the listener, 0 to 65535 [unconfirmed]
        0,                             // 5 reserved [unconfirmed]
        distance_word(inputs.distance, inputs.in_front), // 6 signed distance, metres [decomp range]
        RANGE_CM,                      // 7 range, cm [unconfirmed]
        RANGE_CM,                      // 8 range, cm [unconfirmed]
        32767,                         // 9 full scale [unconfirmed]
        0,                             // 10 reserved [unconfirmed]
        speed_word(inputs.forward_speed), // 11 speed, 0 to 1023 [decomp]
    ]
}
