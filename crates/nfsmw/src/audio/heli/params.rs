//! The rotor loop's class words and the pure mappings behind them. Spec: `docs/specs/ai-helicopter-rotor.md`.
//! The clamps of the twelve words are the original's publisher (`[decomp]`). What the `FX_Helicopter` module does
//! with each word was measured by running the installed module (`[measured]`, section 2 of that spec). The map
//! already does the distance rolloff and the rotor's pitch, so volume and pitch pass through.

/// The low-pass word with the filter open, Hz (the publisher's clamp). [measured: the module passes it to the low-pass]
const LOW_PASS_OPEN: i32 = 25000;
/// The dry level, full scale, as the other world modules give it. [unconfirmed: the module does not read the word]
const DRY: i32 = 32767;
/// The publisher clamps the pitch word to plus or minus this. [decomp]
const PITCH_CLAMP: i32 = 8192;
/// The publisher clamps the signed distance to plus or minus this. [decomp]
const DISTANCE_CLAMP: f32 = 100.0;
/// The speed that reads as the full speed word: the fraction is `speed / 500`. [decomp]
const FULL_SPEED: f32 = 500.0;
/// Below this fraction the speed fraction is the idle value. [decomp: the test is on the fraction, so any helicopter
/// slower than 125 m/s gives the idle word]
const IDLE_BELOW: f32 = 0.25;
const IDLE_SPEED: f32 = 0.5;
/// The rotation word's full range: half a turn of the original's 16-bit angle, shifted down by 6. [decomp]
const ROTATION_FULL: f32 = 512.0;

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
    /// The helicopter's speed, m/s.
    pub speed: f32,
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

/// The speed fraction: `speed / 500`, at most 1, and the idle 0.5 when the fraction is under 0.25. [decomp]
pub fn speed_fraction(speed: f32) -> f32 {
    if !speed.is_finite() {
        return IDLE_SPEED;
    }
    let fraction = speed / FULL_SPEED;
    if fraction > 1.0 {
        return 1.0;
    }
    if fraction < IDLE_BELOW {
        return IDLE_SPEED;
    }
    fraction
}

/// The speed word, 0 to 1023. [decomp]
pub fn speed_word(speed: f32) -> i32 {
    (speed_fraction(speed) * 1023.0) as i32
}

/// The rotation word: the bearing over half a turn, 0 (the nose points at the listener) to 512 (away). [decomp]
pub fn rotation_word(bearing: f32) -> i32 {
    if !bearing.is_finite() {
        return 0;
    }
    (bearing.clamp(0.0, std::f32::consts::PI) / std::f32::consts::PI * ROTATION_FULL) as i32
}

/// The twelve class words of `FX_Helicopter`, in the order the publisher stores them (word 0 at byte +4 of the
/// original's class object), clamped as the publisher clamps them. [decomp] for the clamps, [measured] for the use.
pub fn class_words(inputs: RotorInputs) -> [i32; 12] {
    [
        0,                                               // 0 not read by the module
        inputs.pitch.clamp(-PITCH_CLAMP, PITCH_CLAMP),   // 1 pitch, Q12: every layer's pitch
        i32::from(inputs.in_front),                      // 2 not read by the module
        inputs.volume.clamp(0, 32767),                   // 3 volume: every layer's level
        i32::from(inputs.azimuth),                       // 4 azimuth, passed to every layer
        rotation_word(inputs.bearing).clamp(0, 1024),    // 5 rotation: picks the blade samples
        distance_word(inputs.distance, inputs.in_front), // 6 signed distance: near, far and wash layers
        LOW_PASS_OPEN,                                   // 7 low-pass, Hz
        0,                                               // 8 not read by the module
        DRY,                                             // 9 not read by the module
        0,                                               // 10 reverb send
        speed_word(inputs.speed),                        // 11 speed: the fast loop against the hover layers
    ]
}
