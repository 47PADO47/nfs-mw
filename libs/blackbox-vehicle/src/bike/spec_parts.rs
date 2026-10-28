//! The parts of a bike's spec that sit under the wheels and the engine: suspension, engine and gearbox. SI units.

/// A progressive spring term: the force law adds `factor * sgn(x) * max(|x| - start * travel, 0)^linearity` (with `x` in cm
/// inside the formula, see `suspension.rs`) to the compression before the rate multiplies it. A `factor` of 0 switches it
/// off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progressive {
    /// Where the progressive part starts, as a share of the maximum travel.
    pub start: f32,
    pub factor: f32,
    pub linearity: f32,
}

/// The second, progressive spring-like term (the hydraulic bump stop). A `stiffness` of 0 switches it off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hydraulic {
    /// Rate (N/m).
    pub stiffness: f32,
    pub term: Progressive,
}

/// The forces that couple the drive, the tyre and the brake into the suspension (`docs/specs/ride6-suspension.md`): each is a
/// factor times a remap of the compression share `s` from 0 to 1, `out.0` at no compression to `out.1` at full travel. A factor
/// of 0 switches it off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coupling {
    /// Chain pull: newtons per newton metre of drive torque at the wheel, in units of 100 (a factor of -0.014 adds 1.4 N per
    /// N m, resisting compression: anti-squat).
    pub chain: f32,
    pub chain_range: [f32; 2],
    /// The share of the tyre's longitudinal force that loads the suspension.
    pub traction: f32,
    pub traction_range: [f32; 2],
    /// The brake coupling: the factor over 100 times the brake force at the patch.
    pub brake: f32,
    pub brake_range: [f32; 2],
}

/// A fork or a shock with its damper.
#[derive(Clone, Debug)]
pub struct SuspensionSpec {
    /// The length the suspension can compress over from its fully extended state, at the wheel (m).
    pub max_travel: f32,
    /// Spring rate at the shock (N/m).
    pub spring: f32,
    /// Stiffness of the spring that opposes full extension (N/m), acting over the first `top_out_length` of the travel.
    pub top_out_stiffness: f32,
    /// Length of the top-out zone (m).
    pub top_out_length: f32,
    pub damper: DamperSpec,
    /// `(share of travel, ratio)` of wheel travel to shock travel; empty for a spring that acts on the wheel directly.
    pub leverage: Vec<(f32, f32)>,
    /// Compression added to the spring's input as a preload (m).
    pub rest_offset: f32,
    pub progressive: Progressive,
    pub hydraulic: Hydraulic,
    /// The largest compression speed the force law sees (m/s); 0 for no limit.
    pub max_speed: f32,
    pub coupling: Coupling,
}

/// Two-slope damper: the slow slope up to a threshold speed, the fast slope beyond it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamperSpec {
    pub slow_bump: f32,
    pub fast_bump: f32,
    pub bump_threshold: f32,
    pub slow_rebound: f32,
    pub fast_rebound: f32,
    pub rebound_threshold: f32,
}

#[derive(Clone, Debug)]
pub struct EngineSpec {
    /// The peak torque (N m) the torque curve's factors multiply.
    pub peak_torque: f32,
    /// `(rpm, share of the peak)`, rpm rising.
    pub torque_curve: Vec<(f32, f32)>,
    /// Drag torque of the closed throttle at the rev limit (N m), times the friction curve.
    pub friction_torque: f32,
    /// `(rpm / max_rpm, factor)` of the drag; empty means the factor is the share of the rev limit itself.
    pub friction_curve: Vec<(f32, f32)>,
    /// The engine speed below which the throttle is held at `throttle_at_idle` (rpm).
    pub idle_rpm: f32,
    pub max_rpm: f32,
    /// Moment of inertia of the rotating engine (kg m^2).
    pub moi: f32,
    /// The least throttle below the idle speed; 0 for none.
    pub throttle_at_idle: f32,
    /// How long the rev limiter cuts and restores the fuel in turn (s).
    pub limiter_cut_time: f32,
}

#[derive(Clone, Debug)]
pub struct TransmissionSpec {
    /// Forward gear ratios, first gear first.
    pub gears: Vec<f32>,
    /// The ratio between the gearbox output and the rear wheel, chain included.
    pub final_ratio: f32,
    /// The share of the engine torque that reaches the wheel.
    pub efficiency: f32,
    /// The engine speed, as a share of the rev limit, at which the automatic changes up under throttle.
    pub up_shift: f32,
    /// The engine speed, as a share of the rev limit, below which it changes down.
    pub down_shift: f32,
    /// How long a change up and a change down take (s).
    pub up_shift_time: f32,
    pub down_shift_time: f32,
    /// The throttle blip on a change down.
    pub blip_throttle: f32,
    /// The exponent of the clutch's engagement.
    pub clutch_linearity: f32,
    /// The back torque range (N m) over which the slipper clutch opens, and how far it opens (0 to 1; 0 for no slipper).
    pub slipper_range: [f32; 2],
    pub slipper_intensity: f32,
    /// The road speed (m/s) above which a start is over and the clutch is locked; 0 for no launch slip.
    pub release_speed: f32,
    /// The engine speed of a launch, as a share of the engine speed of peak torque, and the most it may be as a share of the
    /// rev limit.
    pub feather_factor: f32,
    pub feather_max_factor: f32,
    /// The most torque the clutch carries, as a multiple of the engine's peak torque.
    pub clutch_max_factor: f32,
}
