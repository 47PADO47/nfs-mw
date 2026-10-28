//! The parameters of the rider help around a wheelie or a stoppie: the throttle and brake interventions and the auto-rider
//! that moves the rider's weight (`docs/specs/ride6-tricks.md`, sections 4 and 5). A field the data of a bike does not set
//! takes the value of `Default`, which are the constructor defaults of the reference (recovered).

use std::f32::consts::FRAC_PI_2;

/// The rider help's throttle and brake interventions, its intensity and the auto-rider.
#[derive(Clone, Debug)]
pub struct RiderHelpSpec {
    /// Below this speed (m/s) the throttle cut blends from its launch values to its normal ones; 0 or less switches the
    /// blend off (only the normal values apply).
    pub launch_speed: f32,
    /// The lift angle range (rad) of the throttle cut at a standstill, and the factor on its strength there.
    pub launch_throttle_pitch: [f32; 2],
    pub launch_throttle_multiplier: f32,
    /// The factor on the strength of the throttle cut above the launch speed.
    pub throttle_multiplier: f32,
    /// The lift angle range (rad) of the front brake's relief while the rider asks for a stoppie, and the factor on its
    /// strength otherwise.
    pub forced_brake_pitch: [f32; 2],
    pub brake_multiplier: f32,
    /// Whether an intervention acts on the engine's throttle modulation (the electronic one) instead of the pedal itself.
    /// The pedal feeds the wheelie target, the modulation does not. Per intervention: the lift angle and the lift speed, for
    /// the throttle and for the front brake.
    pub throttle_pitch_electronic: bool,
    pub throttle_speed_electronic: bool,
    pub brake_pitch_electronic: bool,
    pub brake_speed_electronic: bool,
    /// The strength of all the interventions (`AutoRiderIntensity`).
    pub intensity: f32,
    /// The rider's weight input (0 to 1) above which the player has the weight and the auto-rider stands aside.
    pub weight_deadzone: f32,
    /// How far ahead (s) the lift angle is looked at, along its speed, for the interventions and the auto-rider.
    pub lookahead: f32,
    /// The auto-rider; `None` leaves the weight to the player.
    pub auto_rider: Option<AutoRiderSpec>,
}

impl Default for RiderHelpSpec {
    fn default() -> Self {
        let wide = [30.0_f32.to_radians(), FRAC_PI_2];
        Self {
            launch_speed: 0.0,
            launch_throttle_pitch: wide,
            launch_throttle_multiplier: 1.0,
            throttle_multiplier: 1.0,
            forced_brake_pitch: wide,
            brake_multiplier: 1.0,
            throttle_pitch_electronic: false,
            throttle_speed_electronic: false,
            brake_pitch_electronic: false,
            brake_speed_electronic: false,
            intensity: 1.0,
            weight_deadzone: 0.8,
            lookahead: 0.0,
            auto_rider: None,
        }
    }
}

/// The auto-rider's weight control: it shifts the rider's weight against a lift (forward against a wheelie, back against a
/// stoppie) and, while the player leaves the weight alone, forward under the throttle and back under the brake.
#[derive(Clone, Debug)]
pub struct AutoRiderSpec {
    /// The lift angle range (rad) over which the weight goes from the player's to the full shift, for a wheelie and for a
    /// stoppie, and the factor on it.
    pub wheelie_range: [f32; 2],
    pub stoppie_range: [f32; 2],
    pub multiplier: f32,
    /// The lift speed range (rad/s) that adds to the shift, with the sign of the speed. A range that descends (the
    /// constructor default) is always 0.
    pub pitch_speed_range: [f32; 2],
    /// The rest position (-1 to 1) the player's hands-off weight takes from the throttle and from the brake: a curve of the
    /// pedal `(pedal, position)`, times a factor.
    pub throttle_curve: Vec<(f32, f32)>,
    pub throttle_gain: f32,
    pub brake_curve: Vec<(f32, f32)>,
    pub brake_gain: f32,
    /// How fast (1/s, a square-root approach) the weight follows its aim while the auto-rider has it, and while the player
    /// has it; 0 for at once.
    pub engage_speed: f32,
    pub manual_engage_speed: f32,
}

impl Default for AutoRiderSpec {
    fn default() -> Self {
        Self {
            wheelie_range: [0.0, 0.0],
            stoppie_range: [0.0, 0.0],
            multiplier: 1.0,
            pitch_speed_range: [0.0, -1.0],
            throttle_curve: Vec::new(),
            throttle_gain: 1.0,
            brake_curve: Vec::new(),
            brake_gain: 1.0,
            engage_speed: 0.0,
            manual_engage_speed: 0.0,
        }
    }
}
