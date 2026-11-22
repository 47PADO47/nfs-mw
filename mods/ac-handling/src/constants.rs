//! The fixed constants of the model (docs/specs/ac-handling/tyre-force.md section 5 and wheel-dynamics.md section 5): the
//! numbers that are not keys of a preset. Origin codes as in the specs: [U] understood from the reference, [D] design value.

/// `fz0`, N: the reference load at which `mu0` is stated. [U]
pub const FZ0: f64 = 2000.0;
/// Normalised slip at which the pneumatic trail has finished falling. [U]
pub const TRAIL_N_END: f64 = 1.25;
/// The final trail as a negative share of the trail at zero slip. [U]
pub const TRAIL_UNDER: f64 = 0.10;
/// How much of a grip change moves the peak slip. [U]
pub const PEAK_FOLLOW: f64 = 0.75;
/// Patch speed (m/s) below which the aligning torque is faded out. [U]
pub const TRAIL_FADE_SPEED: f64 = 1.0;
/// Cap of the total grip multiplier of one direction. [D]
pub const GRIP_MAX: f64 = 10.0;
/// Substeps per second of simulated time. [D]
pub const SUBSTEPS_PER_SECOND: f64 = 600.0;
/// The most substeps one tick takes. [D]
pub const MAX_SUBSTEPS: u32 = 24;
/// Step of the central difference that estimates the slope of the longitudinal force. [D]
pub const SLOPE_STEP: f64 = 1.0e-3;
/// Floor of the grip relaxation length factor. [D]
pub const RELAX_LENGTH_FLOOR: f64 = 0.3;
/// The longest tick (s) the model accepts; a longer one is left to the built-in tyre. [D]
pub const MAX_DT: f64 = 0.1;
/// The game's reference tyre on its `STATIC_GRIP` scale: grip level 1.0. [N]
pub const REFERENCE_PEAK_FRICTION: f64 = 2.075;
/// Sliding over peak friction of a car with no usable figures. [D]
pub const DEFAULT_RHO: f64 = 0.89;
