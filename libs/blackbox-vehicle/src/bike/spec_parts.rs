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
    /// The tolerances (per gear) that scale the torque the generated change points compare: the next gear has to win by this
    /// factor (up) or lose by it (down). Shorter lists count as 1.
    pub up_tolerance: Vec<f32>,
    pub down_tolerance: Vec<f32>,
    /// The hold-offs of the automatic changes, each `(factor, linearity)` of `normalised^linearity x factor`: lean holds the
    /// change up back, lean and the throttle hold the change down back.
    pub prevent_up_lean: [f32; 2],
    pub prevent_down_lean: [f32; 2],
    pub prevent_down_throttle: [f32; 2],
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
    pub feather: FeatherSpec,
    /// The launch clutch when there is no feathering: the shares of the rev limit between which it comes in.
    pub launch_rpm: [f32; 2],
    /// The most torque the clutch carries, as a multiple of the engine's peak torque.
    pub clutch_max_factor: f32,
    /// How a change moves the clutch and the throttle, the idle clutch and the slipper filter (`ClutchControl::default()`
    /// when the manifest leaves them out).
    pub control: super::gearbox::ClutchControl,
}

/// The feathering controller of a start (`docs/specs/ride6-gearbox-auto.md`, section 3). `Default` is the constructor default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeatherSpec {
    /// The normalised engine speed at or below which it acts, and above which (with the wheel caught up) it stops.
    pub target_tolerance: f32,
    pub exit_tolerance: f32,
    /// The most throttle the controller lets through.
    pub throttle_cap: f32,
    /// The normalised slip above which it acts.
    pub min_slip: f32,
    /// The correction step (s).
    pub delta_time: f32,
    /// How fast the engine is asked to climb towards the target speed, and how fast the clutch comes back as the wheel
    /// catches up.
    pub rpm_raise: f32,
    pub release_factor: f32,
}

impl Default for FeatherSpec {
    fn default() -> Self {
        Self {
            target_tolerance: 0.6,
            exit_tolerance: 0.8,
            throttle_cap: 1.0,
            min_slip: 0.1,
            delta_time: 0.01,
            rpm_raise: 0.05,
            release_factor: 1.0,
        }
    }
}

/// One strip of the tread (`docs/specs/ride6-tyre-thermal.md`): its heating, cooling and wear constants and the curves that
/// turn its temperature and wear into grip. A missing curve counts as 1.
#[derive(Clone, Debug, Default)]
pub struct TyreSection {
    /// The lowest efficiency the strip may give.
    pub efficiency_limit: f32,
    /// Cooling towards the road and air temperature, heating from sliding, heating per unit of lateral and longitudinal slip
    /// work, heating from rolling resistance.
    pub static_coef: f32,
    pub dynamic_coef: f32,
    pub lat_slip_coef: f32,
    pub long_slip_coef: f32,
    pub rolling_coef: f32,
    /// `(deg C, wear rate)`, `(wear, efficiency)` and `(deg C, efficiency)`.
    pub temperature_to_wear: Vec<(f32, f32)>,
    pub wear_to_efficiency: Vec<(f32, f32)>,
    pub temperature_to_efficiency: Vec<(f32, f32)>,
}

/// The temperature and wear of a tyre (`docs/specs/ride6-tyre-thermal.md`).
#[derive(Clone, Debug, Default)]
pub struct TyreThermal {
    /// The strips of the tread, left to right.
    pub sections: Vec<TyreSection>,
    /// How far across the tread the load spreads over the strips: the dependency and the half-span (rad) of their centres.
    pub sections_dependency: f32,
    pub sections_angle: f32,
    /// The heat transfer between neighbouring strips (0: none).
    pub heat_transfer: f32,
    /// The thickness of a new tread, the temperature the tyre starts at (and its floor), and its ceiling (deg C).
    pub initial_thickness: f32,
    pub min_temperature: f32,
    pub max_temperature: f32,
    /// How much of the steady temperature is the road's, against the air's.
    pub asphalt_importance: f32,
    /// The distance (m) over which a loaded strip keeps the fraction `rate` of its tread.
    pub wear_length: f32,
    /// Every strip takes the mean temperature and thickness after each step.
    pub uniform: bool,
    pub static_time_scale: f32,
    pub dynamic_time_scale: f32,
    pub decay_distance_factor: f32,
    pub wear_intensity: f32,
}
