//! The `engineaudio` class: one engine's loops, banks and mix tuning.
//! Fields and rules: `docs/specs/engine-sound.md` §2, §4 and §5.

use super::fields::{Fields, bezier_points};

/// `eENGINE_GROUP`: the size class of the engine (decides which AI engines share banks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineGroup {
    V4,
    V6,
    V8,
    /// A value outside the known enum.
    Other(u32),
}

impl EngineGroup {
    pub fn from_value(value: u32) -> Self {
        match value {
            0 => Self::V4,
            1 => Self::V6,
            2 => Self::V8,
            other => Self::Other(other),
        }
    }
}

/// A mix level at a steady RPM (`steady`) and at a large RPM change (`large`), 0 to 1. The fields are named
/// `*_S_RPM` and `*_L_RPM` in the data.
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
    /// Sample-layer level while accelerating (`AEMSMix_S_RPM`, `AEMSMix_L_RPM`).
    pub aems: MixLevels,
    /// Accelerate-loop level while accelerating (`GINSUMix_*`).
    pub ginsu: MixLevels,
    /// Sample-layer level off the throttle (`DECEL_AEMSMix_*`).
    pub decel_aems: MixLevels,
    /// Decelerate-loop level off the throttle (`DECEL_GINSUMix_*`).
    pub decel_ginsu: MixLevels,
    /// Accelerate-loop level off the throttle (`Ginsu_ACL_Neg_*`).
    pub ginsu_neg: MixLevels,
    /// RPM change per update at which `large` is reached (`AccelDeltaRPMThreshold`).
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
    /// Fraction of the window over which the loop fades in at the low end (`GINSU_DECEL_FADE_OUT` in the data).
    pub fade_in_fraction: f32,
    /// Fraction of the window over which it fades out at the top (`GINSU_DECEL_FADE_IN` in the data).
    pub fade_out_fraction: f32,
}

impl DecelWindow {
    /// The loop's gain (0 to 1) at `rpm` on the "Ginsu scaled" axis: the 5-point polyline of the spec.
    pub fn gain(&self, rpm: f32) -> f32 {
        let (lo, hi) = (self.min_rpm, self.max_rpm);
        if hi <= lo || rpm <= lo || rpm >= hi {
            return 0.0;
        }
        let width = hi - lo;
        let full_from = lo + width * self.fade_in_fraction;
        let full_to = hi - width * self.fade_out_fraction;
        if rpm < full_from {
            return (rpm - lo) / (full_from - lo);
        }
        if rpm <= full_to {
            return 1.0;
        }
        (hi - rpm) / (hi - full_to)
    }
}

/// One `engineaudio` collection.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineSound {
    /// The collection's name (`tvr_cerb`), or its hash when unnamed.
    pub name: String,
    /// `.gin` file names in `SOUND/ENGINE` (empty when the set has none).
    pub accel_loop: String,
    pub decel_loop: String,
    /// The sample-layer bank (`CAR_66_ENG_MB_EE.abk`).
    pub bank_main: String,
    /// The same sounds for the second memory (`CAR_66_ENG_MB_SPU.abk`).
    pub banks_aux: Vec<String>,
    /// Shift sweeteners and the reverse whine (`SWTN_CAR_66_MB.abk`, `CAR_WHINE_00.abk`).
    pub sweet_banks: Vec<String>,
    /// The id the sample layer knows the engine by (66).
    pub car_id: u32,
    pub group: EngineGroup,
    pub priority: f32,
    pub may_upgrade_to_v8: bool,
    /// The car (when it is the player's) also plays the transmission loop `CAR_TRANNY.abk`.
    pub has_transmission_loop: bool,
    /// The Ginsu frequency range: engine RPM is mapped onto `min_rpm ..= max_rpm`.
    pub min_rpm: f32,
    pub max_rpm: f32,
    /// The `y` of the four Bezier control points that remap the physics RPM fraction (`PhysicsRPM_Map`).
    pub rpm_map: [f32; 4],
    pub master_volume: u32,
    pub mix: EngineMix,
    pub decel_window: DecelWindow,
    pub low_pass_cutoff: u32,
    pub shift_sweet_volume: i32,
    pub sputter_volume: i32,
    pub decel_pitch_offset: f32,
    /// The `acceltrans` collection this engine links to (a name or hash), if any.
    pub accel_transition: Option<String>,
}

impl EngineSound {
    /// Reads one `engineaudio` collection (any of the class's, not only the ones a car links to).
    pub fn from_collection(c: blackbox_attrib::CollectionRef<'_>) -> Self {
        Self::read(Fields(c))
    }

    pub(super) fn read(c: Fields<'_>) -> Self {
        let levels = |steady: &str, large: &str| MixLevels { steady: c.f32(steady), large: c.f32(large) };
        Self {
            name: c.name(),
            accel_loop: c.string("Filename_GinsuAccel"),
            decel_loop: c.string("Filename_GinsuDecel"),
            bank_main: c.string("BankName_mainRAM"),
            banks_aux: c.strings("BankName_auxRAM"),
            sweet_banks: c.strings("SweetBank"),
            car_id: c.u32("CarID"),
            group: EngineGroup::from_value(c.u32("EngType")),
            priority: c.f32("Priority"),
            may_upgrade_to_v8: c.bool("MaybeV8"),
            has_transmission_loop: c.bool("Tranny"),
            min_rpm: c.f32("MinRPM"),
            max_rpm: c.f32("MaxRPM"),
            rpm_map: rpm_map(&c),
            master_volume: c.u32("Master_Vol"),
            mix: EngineMix {
                aems: levels("AEMSMix_S_RPM", "AEMSMix_L_RPM"),
                ginsu: levels("GINSUMix_S_RPM", "GINSUMix_L_RPM"),
                decel_aems: levels("DECEL_AEMSMix_S_RPM", "DECEL_AEMSMix_L_RPM"),
                decel_ginsu: levels("DECEL_GINSUMix_S_RPM", "DECEL_GINSUMix_L_RPM"),
                ginsu_neg: levels("Ginsu_ACL_Neg_S_RPM", "Ginsu_ACL_Neg_L_RPM"),
                accel_delta_threshold: c.f32("AccelDeltaRPMThreshold"),
                decel_delta_threshold: c.f32("DecelDeltaRPMThreshold"),
                aems_volume: c.u32("AEMSVol"),
                decel_aems_volume: c.u32("DECEL_AEMSVol"),
                accel_loop_volume: c.u32("GINSUAccelVol"),
                decel_loop_volume: c.u32("GinsuDecelVol"),
            },
            decel_window: DecelWindow {
                min_rpm: c.u32("GINSU_Decel_MinRPM") as f32,
                max_rpm: c.u32("GINSU_Decel_MaxRPM") as f32,
                fade_in_fraction: c.f32("GINSU_DECEL_FADE_OUT"),
                fade_out_fraction: c.f32("GINSU_DECEL_FADE_IN"),
            },
            low_pass_cutoff: c.u32("GINSU_LowPassCutoff"),
            shift_sweet_volume: c.i32("Vol_ShiftSweets"),
            sputter_volume: c.i32("Vol_Sputters"),
            decel_pitch_offset: c.f32("DecelPitchOffset"),
            accel_transition: c.0.follow("acceltrans").map(|a| Fields(a).name()),
        }
    }

    /// Whether the set has a Ginsu accelerate loop to play.
    pub fn has_ginsu(&self) -> bool {
        !self.accel_loop.is_empty()
    }

    /// The Ginsu target frequency for an engine on the sound scale (1000 to 10000): the spec's
    /// `(rpm - 1000) / 9000 * (MaxRPM - MinRPM) + MinRPM`.
    pub fn ginsu_frequency(&self, sound_rpm: f32) -> f32 {
        let scaled = (sound_rpm.clamp(1000.0, 10000.0) - 1000.0) / 9000.0;
        scaled * (self.max_rpm - self.min_rpm) + self.min_rpm
    }

    /// The audio RPM fraction (0 to 1) for the physics fraction `rpm_pct`: the Bezier remap applied to the
    /// player's car. The sound scale is `1000 + 9000 *` this.
    pub fn remap_rpm(&self, rpm_pct: f32) -> f32 {
        let t = rpm_pct.clamp(0.0, 1.0);
        let u = 1.0 - t;
        let y = self.rpm_map;
        u * u * u * y[0] + 3.0 * t * u * u * y[1] + 3.0 * t * t * u * y[2] + t * t * t * y[3]
    }
}

fn rpm_map(c: &Fields<'_>) -> [f32; 4] {
    let straight = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    let Some(bytes) = c.raw_items("PhysicsRPM_Map").first().copied() else { return straight };
    let Some(points) = bezier_points(bytes) else { return straight };
    points.map(|p| p[1])
}
