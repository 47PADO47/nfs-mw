//! The gearbox and the clutch of the `[bike]` table (`[bike.transmission]`). Same units as `blackbox_vehicle::bike::BikeSpec`.

use blackbox_vehicle::bike::{ClutchControl, FeatherSpec, ForcedDownshift, OverLimit, TransmissionSpec};
use serde::Deserialize;

fn yes() -> bool {
    true
}

fn over_min_speed() -> f32 {
    OverLimit::default().min_speed
}

fn over_change_time() -> f32 {
    OverLimit::default().change_time
}

fn over_threshold() -> f32 {
    OverLimit::default().threshold
}

fn over_share() -> f32 {
    OverLimit::default().rpm_share
}

fn two() -> f32 {
    2.0
}

fn idle_range() -> [f32; 2] {
    ClutchControl::default().min_rpm_speed_range
}

/// The gearbox and the clutch (`[bike.transmission]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transmission {
    /// Forward gear ratios, first gear first.
    pub gears: Vec<f32>,
    /// The ratio between the gearbox output and the rear wheel, chain included.
    pub final_ratio: f32,
    /// The share of the engine torque that reaches the wheel.
    pub efficiency: f32,
    /// The tolerances per gear that scale the torque the generated change points compare (`ride6-gearbox-auto.md`).
    pub up_tolerance: Vec<f32>,
    pub down_tolerance: Vec<f32>,
    /// The hold-offs of the automatic changes, each `[factor, linearity]`; 0 (the default) for none.
    #[serde(default)]
    pub prevent_up_lean: [f32; 2],
    #[serde(default)]
    pub prevent_down_lean: [f32; 2],
    #[serde(default)]
    pub prevent_down_throttle: [f32; 2],
    /// How long a change up and a change down take, s.
    pub up_shift_time_s: f32,
    pub down_shift_time_s: f32,
    /// The throttle blip on a change down; 0 (the default) for none.
    #[serde(default)]
    pub blip_throttle: f32,
    /// The exponent of the clutch's engagement.
    pub clutch_linearity: f32,
    /// The most torque the clutch carries, as a multiple of the engine's peak torque.
    pub clutch_max_factor: f32,
    /// The back torque range in N m over which the slipper clutch opens, and how far it opens (0 to 1); 0 (the default) for
    /// no slipper.
    #[serde(default)]
    pub slipper_range_nm: [f32; 2],
    #[serde(default)]
    pub slipper_intensity: f32,
    /// The road speed in m/s above which a start is over and the clutch is locked; 0 (the default) for no launch slip.
    #[serde(default)]
    pub release_speed_mps: f32,
    /// `ClutchTimingOnGearChange`: fractions `[a, b]` of a gear change. The clutch lever goes to 1 over `[0, a]` (the gear changes
    /// at `a`), stays there up to `b` and goes back to 0 over `[b, 1]`; `[0, 0]` (the default) is a plain ramp back over the change.
    #[serde(default)]
    pub clutch_timing: [f32; 2],
    /// `ThrottleTimingOnGearChange`: fractions `[c, d]`; the throttle falls to 0 over `[0, c]`, stays shut to `d` and comes back
    /// over `[d, 1]`. `[0, 0]` (the default) leaves the throttle to the next two keys.
    #[serde(default)]
    pub throttle_timing: [f32; 2],
    /// `ReleaseThrottleOnShift`: with no throttle timing the throttle is shut for the change.
    #[serde(default)]
    pub release_throttle_on_shift: bool,
    /// `ModulateThrottleOnUpShift` (default true): with no throttle timing the throttle follows the clutch on a change up.
    #[serde(default = "yes")]
    pub modulate_throttle_on_upshift: bool,
    /// `PressClutchAtMinRPM` (default true) and `MinRPMSpeedScaleRange` (default `[1.05, 1.1]`): the clutch is pulled as the
    /// throttle-shut bike slows to the road speed at which the engine would turn at its idle speed.
    #[serde(default = "yes")]
    pub press_clutch_at_min_rpm: bool,
    #[serde(default = "idle_range")]
    pub min_rpm_speed_scale_range: [f32; 2],
    /// `SlipperClutchFilterSpeed` (default 0: no filter).
    #[serde(default)]
    pub slipper_filter_speed: f32,
    /// The forced downshift under hard braking (`ForceGearDownChangeBrakeInputRange`, `...Duration`, `...Factor`, `...Linearity`,
    /// `ForceGearDownChangeSpeedInputRange`): the front lever range, the time (s), the strength factor (0, the default, is off),
    /// the exponent (default 2) and the road speed range (m/s).
    #[serde(default)]
    pub force_downshift_brake_range: [f32; 2],
    #[serde(default)]
    pub force_downshift_duration_s: f32,
    #[serde(default)]
    pub force_downshift_factor: f32,
    #[serde(default = "two")]
    pub force_downshift_linearity: f32,
    #[serde(default)]
    pub force_downshift_speed_range_mps: [f32; 2],
    /// The over-limit upshift (`GearChangeOverLimitEnabled`, `OverLimitMinChangeSpeed`, `OverLimitChangeTime`,
    /// `OverLimitChangeTimeThreshold`, `OverLimitRpmPercentage`): on by default, the speed in m/s, the times in s and the share of
    /// the rev limit; the defaults are the reference's constructor defaults.
    #[serde(default = "yes")]
    pub over_limit_enabled: bool,
    #[serde(default = "over_min_speed")]
    pub over_limit_min_speed_mps: f32,
    #[serde(default = "over_change_time")]
    pub over_limit_change_time_s: f32,
    #[serde(default = "over_threshold")]
    pub over_limit_threshold_s: f32,
    #[serde(default = "over_share")]
    pub over_limit_rpm_share: f32,
    /// The engine speed of a launch, as a share of the engine speed of peak torque, and the most it may be as a share of the
    /// rev limit.
    #[serde(default)]
    pub feather_factor: f32,
    #[serde(default)]
    pub feather_max_factor: f32,
    /// The rest of the feathering controller (the constructor defaults when left out).
    #[serde(default)]
    pub feather: Feather,
    /// The launch clutch without feathering: the shares of the rev limit between which it comes in.
    #[serde(default)]
    pub launch_rpm: [f32; 2],
}

/// The feathering controller of a start (`[bike.transmission.feather]`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Feather {
    pub target_tolerance: f32,
    pub exit_tolerance: f32,
    pub throttle_cap: f32,
    pub min_slip: f32,
    pub delta_time_s: f32,
    pub rpm_raise: f32,
    pub release_factor: f32,
}

impl Default for Feather {
    fn default() -> Self {
        let d = FeatherSpec::default();
        Self {
            target_tolerance: d.target_tolerance,
            exit_tolerance: d.exit_tolerance,
            throttle_cap: d.throttle_cap,
            min_slip: d.min_slip,
            delta_time_s: d.delta_time,
            rpm_raise: d.rpm_raise,
            release_factor: d.release_factor,
        }
    }
}

impl Transmission {
    pub(super) fn to_spec(&self) -> TransmissionSpec {
        TransmissionSpec {
            gears: self.gears.clone(),
            final_ratio: self.final_ratio,
            efficiency: self.efficiency,
            up_tolerance: self.up_tolerance.clone(),
            down_tolerance: self.down_tolerance.clone(),
            prevent_up_lean: self.prevent_up_lean,
            prevent_down_lean: self.prevent_down_lean,
            prevent_down_throttle: self.prevent_down_throttle,
            up_shift_time: self.up_shift_time_s,
            down_shift_time: self.down_shift_time_s,
            blip_throttle: self.blip_throttle,
            clutch_linearity: self.clutch_linearity,
            slipper_range: self.slipper_range_nm,
            slipper_intensity: self.slipper_intensity,
            release_speed: self.release_speed_mps,
            feather_factor: self.feather_factor,
            feather_max_factor: self.feather_max_factor,
            feather: FeatherSpec {
                target_tolerance: self.feather.target_tolerance,
                exit_tolerance: self.feather.exit_tolerance,
                throttle_cap: self.feather.throttle_cap,
                min_slip: self.feather.min_slip,
                delta_time: self.feather.delta_time_s,
                rpm_raise: self.feather.rpm_raise,
                release_factor: self.feather.release_factor,
            },
            launch_rpm: self.launch_rpm,
            clutch_max_factor: self.clutch_max_factor,
            control: ClutchControl {
                clutch_timing: self.clutch_timing,
                throttle_timing: self.throttle_timing,
                release_throttle: self.release_throttle_on_shift,
                modulate_upshift_throttle: self.modulate_throttle_on_upshift,
                press_at_min_rpm: self.press_clutch_at_min_rpm,
                min_rpm_speed_range: self.min_rpm_speed_scale_range,
                slipper_filter_speed: self.slipper_filter_speed,
                forced_downshift: ForcedDownshift {
                    brake_range: self.force_downshift_brake_range,
                    duration: self.force_downshift_duration_s,
                    factor: self.force_downshift_factor,
                    linearity: self.force_downshift_linearity,
                    speed_range: self.force_downshift_speed_range_mps,
                },
                over_limit: OverLimit {
                    enabled: self.over_limit_enabled,
                    min_speed: self.over_limit_min_speed_mps,
                    change_time: self.over_limit_change_time_s,
                    threshold: self.over_limit_threshold_s,
                    rpm_share: self.over_limit_rpm_share,
                },
            },
        }
    }
}
