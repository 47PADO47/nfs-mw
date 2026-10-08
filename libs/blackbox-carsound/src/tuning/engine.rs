//! The `engineaudio` fields the engine mix reads. Spec: `docs/specs/engine-sound.md` §4 to §7.

/// Whether the car plays one Ginsu loop or two (accelerate and decelerate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineMode {
    Single,
    #[default]
    Dual,
}

/// A mix level at a steady RPM (`steady`) and at a large RPM change (`large`), 0 to 1 (`*_S_RPM`, `*_L_RPM`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MixLevels {
    pub steady: f32,
    pub large: f32,
}

impl MixLevels {
    /// `steady + t * (large - steady)`.
    pub fn at(&self, t: f32) -> f32 {
        self.steady + t * (self.large - self.steady)
    }
}

/// The levels and volumes that blend the sample layer ("AEMS") and the Ginsu loops.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EngineMix {
    /// Sample-layer level while accelerating (`AEMSMix_*`).
    pub aems: MixLevels,
    /// Accelerate-loop level while accelerating (`GINSUMix_*`).
    pub ginsu: MixLevels,
    /// Sample-layer level off the throttle (`DECEL_AEMSMix_*`).
    pub decel_aems: MixLevels,
    /// Decelerate-loop level off the throttle (`DECEL_GINSUMix_*`).
    pub decel_ginsu: MixLevels,
    /// Accelerate-loop level off the throttle (`Ginsu_ACL_Neg_*`).
    pub ginsu_neg: MixLevels,
    /// RPM change per tick at which `large` is reached (`AccelDeltaRPMThreshold`).
    pub accel_delta_threshold: f32,
    /// The same off the throttle (`DecelDeltaRPMThreshold`).
    pub decel_delta_threshold: f32,
    /// Full-scale volumes, 0 to 32767 (`AEMSVol`, `DECEL_AEMSVol`, `GINSUAccelVol`, `GinsuDecelVol`).
    pub aems_volume: u32,
    pub decel_aems_volume: u32,
    pub accel_loop_volume: u32,
    pub decel_loop_volume: u32,
}

/// The RPM window in which the decelerate loop plays (`GINSU_Decel_MinRPM`, `_MaxRPM`, `GINSU_DECEL_FADE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DecelWindow {
    pub min_rpm: f32,
    pub max_rpm: f32,
    /// Fraction of the window over which the loop fades in at the low end (`GINSU_DECEL_FADE_OUT`).
    pub fade_in_fraction: f32,
    /// Fraction of the window over which it fades out at the top (`GINSU_DECEL_FADE_IN`).
    pub fade_out_fraction: f32,
}

impl DecelWindow {
    /// The loop's gain (0 to 1) at `rpm` on the Ginsu frequency axis: the 5-point polyline of the spec.
    pub fn gain(&self, rpm: f32) -> f32 {
        let (lo, hi) = (self.min_rpm, self.max_rpm);
        if hi.is_nan() || lo.is_nan() || hi <= lo || rpm.is_nan() || rpm <= lo || rpm >= hi {
            return 0.0;
        }
        let width = hi - lo;
        let full_from = lo + width * self.fade_in_fraction.clamp(0.0, 1.0);
        let full_to = hi - width * self.fade_out_fraction.clamp(0.0, 1.0);
        if rpm < full_from {
            return (rpm - lo) / (full_from - lo);
        }
        if rpm <= full_to {
            return 1.0;
        }
        (hi - rpm) / (hi - full_to)
    }
}

/// One `engineaudio` collection, as far as the controllers need it.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineTuning {
    pub mode: EngineMode,
    /// The Ginsu frequency range: engine RPM is mapped onto `min_rpm ..= max_rpm` (`MinRPM`, `MaxRPM`).
    pub min_rpm: f32,
    pub max_rpm: f32,
    /// The `y` of the four Bezier control points that remap the physics RPM fraction (`PhysicsRPM_Map`).
    pub rpm_map: [f32; 4],
    pub mix: EngineMix,
    pub decel_window: DecelWindow,
    /// `GINSU_LowPassCutoff`, Hz.
    pub low_pass_cutoff: u32,
    /// `Vol_ShiftSweets`: level of the shift sweeteners, 0 to 32767.
    pub shift_sweet_volume: i32,
    /// `Vol_Sputters`: level of the backfire, 0 to 32767 (carried for the game; no sound is made here).
    pub sputter_volume: i32,
    /// Lowest frequency of the accelerate `.gin` file (its `min_frequency`); 0 when unknown. Below it the loops
    /// play at that minimum and the playback rate carries the ratio (spec §6). The decelerate loop gets the
    /// same two values: the original takes them from the accelerate loop only.
    pub accel_loop_min_frequency: f32,
    /// Duck the engine at the limiter (not for AI racers).
    pub redline_enabled: bool,
}

impl Default for EngineTuning {
    fn default() -> Self {
        Self {
            mode: EngineMode::Dual,
            min_rpm: 1000.0,
            max_rpm: 9000.0,
            rpm_map: [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0],
            mix: EngineMix::default(),
            decel_window: DecelWindow::default(),
            low_pass_cutoff: 25000,
            shift_sweet_volume: 0,
            sputter_volume: 0,
            accel_loop_min_frequency: 0.0,
            redline_enabled: true,
        }
    }
}
