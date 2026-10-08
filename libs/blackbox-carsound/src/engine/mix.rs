//! The hybrid motor: the Ginsu target frequency and the volumes of the engine layers. Spec:
//! `docs/specs/engine-sound.md` §5.

use super::accel_trans::{AccelState, AccelTrans};
use super::rpm::EngineCtl;
use super::shifting::Shifting;
use super::{ShiftState, TickContext};
use crate::avg::RunningAverage;
use crate::math::{finite, lerp, ramp, slew};
use crate::rng::Rng;
use crate::tuning::{EngineMode, EngineTuning};

/// Offset added to the RPM change before it is compared with the thresholds.
const DELTA_RPM_OFFSET: f32 = 10.0;
/// Throttle (percent) at which the mix is fully the accelerate mix.
const TORQUE_THRESHOLD: f32 = 35.0;
/// Largest volume change per tick (Q15).
const MAX_VOLUME_STEP: f32 = 7000.0;
/// Largest mix level change per tick.
const MIX_STEP: f32 = 0.2;
/// Largest low-pass change per tick (Hz).
const CUTOFF_STEP: f32 = 6000.0;
/// Cutoff of the accelerate loop: fully open.
const OPEN_CUTOFF: f32 = 25000.0;
/// Speed (mph) and RPM change that count as steady cruising, and how long it must last (seconds).
const STEADY_SPEED_MPH: f32 = 30.0;
const STEADY_DELTA_RPM: f32 = 30.0;
const STEADY_SECONDS: f32 = 3.0;

/// The mix levels of one tick (0..1) and the low-pass.
#[derive(Debug, Clone, Copy, Default)]
struct Levels {
    aems: f32,
    accel: f32,
    decel: f32,
    cutoff: f32,
}

/// The engine layers' volumes (Q15, unscaled by the redline).
#[derive(Debug, Clone, Copy, Default)]
struct Volumes {
    aems: f32,
    accel: f32,
    decel: f32,
}

/// What the hybrid motor hands out each tick.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MixOutput {
    pub ginsu_frequency: f32,
    /// Q15 volumes after the redline factors.
    pub aems_volume: f32,
    pub accel_volume: f32,
    pub decel_volume: f32,
    pub redline_volume: f32,
    pub low_pass_hz: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HybridMix {
    avg_delta: RunningAverage<4>,
    prev_phys_rpm: Option<f32>,
    levels: Levels,
    volumes: Volumes,
    steady_since: f32,
    bump_countdown: u32,
    pub output: MixOutput,
}

impl HybridMix {
    pub fn new() -> Self {
        Self {
            avg_delta: RunningAverage::flushed(0.0),
            prev_phys_rpm: None,
            levels: Levels { cutoff: OPEN_CUTOFF, ..Levels::default() },
            volumes: Volumes::default(),
            steady_since: 0.0,
            bump_countdown: 0,
            output: MixOutput::default(),
        }
    }

    pub fn update(
        &mut self,
        ctx: &TickContext<'_>,
        eng: &EngineCtl,
        shift: &Shifting,
        accel: &AccelTrans,
        tuning: &EngineTuning,
    ) {
        self.update_delta(ctx, eng, shift, accel);
        let clamped = finite(eng.rpm).clamp(1000.0, 10000.0);
        let frequency = (clamped - 1000.0) / 9000.0 * (tuning.max_rpm - tuning.min_rpm) + tuning.min_rpm;
        self.output.ginsu_frequency = finite(frequency);
        let smoothing = self.smoothing(ctx, shift, accel, tuning.mode);
        let (new_levels, new_volumes) = match tuning.mode {
            EngineMode::Single => self.single_mix(eng, tuning),
            EngineMode::Dual => self.dual_mix(eng, tuning),
        };
        self.apply(smoothing, new_levels, new_volumes);
        self.scale_for_redline(eng, tuning);
    }

    fn update_delta(&mut self, ctx: &TickContext<'_>, eng: &EngineCtl, shift: &Shifting, accel: &AccelTrans) {
        let phys_rpm = ctx.physics.rpm;
        let prev = self.prev_phys_rpm.unwrap_or(phys_rpm);
        self.prev_phys_rpm = Some(phys_rpm);
        let phys_delta = phys_rpm - prev;
        let audio_delta = eng.rpm - eng.prev_rpm;
        if accel.active() {
            let value = if accel.state == AccelState::Attack { phys_delta } else { audio_delta };
            self.avg_delta.flush(value);
            return;
        }
        if !shift.active() {
            self.avg_delta.record(phys_delta);
            return;
        }
        if shift.state == ShiftState::DownEngagingRise {
            self.avg_delta.flush(audio_delta);
            return;
        }
        if shift.stage_changed == ShiftState::UpEngaging {
            self.avg_delta.flush(phys_delta.abs());
            return;
        }
        if matches!(shift.state, ShiftState::UpEngaging | ShiftState::UpLfo) {
            self.avg_delta.record(audio_delta.abs());
            return;
        }
        self.avg_delta.record(audio_delta);
    }

    /// Whether the mix moves toward its target gradually; the engage of an up shift and the attack of a
    /// throttle stab take the new values at once.
    fn smoothing(&self, ctx: &TickContext<'_>, shift: &Shifting, accel: &AccelTrans, mode: EngineMode) -> bool {
        if ctx.physics.gear_changed() || shift.active() {
            return match shift.state {
                ShiftState::UpEngaging => false,
                ShiftState::DownEngagingReattach => !(mode == EngineMode::Single && ctx.physics.accelerating),
                _ => true,
            };
        }
        !(accel.active() && accel.state == AccelState::Attack)
    }

    fn fractions(&self, tuning: &EngineTuning) -> (f32, f32) {
        let delta = (self.avg_delta.value() + DELTA_RPM_OFFSET).abs();
        (
            threshold_fraction(delta, tuning.mix.accel_delta_threshold),
            threshold_fraction(delta, tuning.mix.decel_delta_threshold),
        )
    }

    fn single_mix(&self, eng: &EngineCtl, tuning: &EngineTuning) -> (Levels, Volumes) {
        let mix = &tuning.mix;
        let load = ramp(eng.torque, 0.0, TORQUE_THRESHOLD, 0.0, 1.0);
        let (pct_acc, pct_dec) = self.fractions(tuning);
        let acc = Levels { aems: mix.aems.at(pct_acc), accel: mix.ginsu.at(pct_acc), decel: 0.0, cutoff: OPEN_CUTOFF };
        let dec = Levels {
            aems: mix.decel_aems.at(pct_dec),
            accel: mix.ginsu_neg.steady,
            decel: 0.0,
            cutoff: tuning.low_pass_cutoff as f32,
        };
        let levels = blend(&dec, &acc, load);
        let vf = (eng.volume / 32767.0).max(0.0);
        let volumes = Volumes {
            aems: levels.aems * mix.aems_volume as f32 * vf,
            accel: levels.accel * mix.accel_loop_volume as f32 * vf,
            decel: 0.0,
        };
        (levels, volumes)
    }

    fn dual_mix(&self, eng: &EngineCtl, tuning: &EngineTuning) -> (Levels, Volumes) {
        let mix = &tuning.mix;
        let load = ramp(eng.torque, 0.0, TORQUE_THRESHOLD, 0.0, 1.0);
        let (pct_acc, pct_dec) = self.fractions(tuning);
        let acc = Levels { aems: mix.aems.at(pct_acc), accel: mix.ginsu.at(pct_acc), decel: 0.0, cutoff: OPEN_CUTOFF };
        let window = tuning.decel_window.gain(self.output.ginsu_frequency);
        let dec = Levels {
            aems: (1.0 - window) + window * mix.decel_aems.at(pct_dec),
            accel: mix.ginsu_neg.steady,
            decel: window * mix.decel_ginsu.at(pct_dec),
            cutoff: tuning.low_pass_cutoff as f32,
        };
        let levels = blend(&dec, &acc, load);
        let vf = (eng.volume / 32767.0).max(0.0);
        let aems_volume = lerp(mix.decel_aems_volume as f32, mix.aems_volume as f32, load);
        let volumes = Volumes {
            aems: levels.aems * aems_volume * vf,
            accel: levels.accel * mix.accel_loop_volume as f32 * vf,
            decel: levels.decel * mix.decel_loop_volume as f32 * vf,
        };
        (levels, volumes)
    }

    fn apply(&mut self, smoothing: bool, levels: Levels, volumes: Volumes) {
        if !smoothing {
            self.levels = levels;
            self.volumes = volumes;
            return;
        }
        self.levels = Levels {
            aems: slew(self.levels.aems, levels.aems, MIX_STEP),
            accel: slew(self.levels.accel, levels.accel, MIX_STEP),
            decel: slew(self.levels.decel, levels.decel, MIX_STEP),
            cutoff: slew(self.levels.cutoff, levels.cutoff, CUTOFF_STEP),
        };
        self.volumes = Volumes {
            aems: slew(self.volumes.aems, volumes.aems, MAX_VOLUME_STEP),
            accel: slew(self.volumes.accel, volumes.accel, MAX_VOLUME_STEP),
            decel: slew(self.volumes.decel, volumes.decel, MAX_VOLUME_STEP),
        };
    }

    fn scale_for_redline(&mut self, eng: &EngineCtl, tuning: &EngineTuning) {
        let engine = eng.eng_factor.value();
        let vf = (eng.volume / 32767.0).max(0.0);
        self.output.aems_volume = self.volumes.aems * engine;
        self.output.accel_volume = self.volumes.accel * engine;
        self.output.decel_volume = self.volumes.decel * engine;
        self.output.redline_volume = tuning.mix.aems_volume as f32 * vf * eng.samp_factor.value();
        self.output.low_pass_hz = self.levels.cutoff;
    }

    /// Starts a compression bump now and then while the car cruises at a steady RPM above 30 mph.
    pub fn update_cruise(&mut self, ctx: &TickContext<'_>, eng: &mut EngineCtl, rng: &mut Rng) {
        let steady = ctx.speed_mph > STEADY_SPEED_MPH && self.avg_delta.value().abs() < STEADY_DELTA_RPM;
        if !steady {
            self.steady_since = ctx.now;
            return;
        }
        if ctx.now <= self.steady_since + STEADY_SECONDS {
            return;
        }
        if self.bump_countdown == 0 {
            self.bump_countdown = 60 + rng.below(150);
            eng.play_compression = true;
        }
        self.bump_countdown -= 1;
    }
}

/// `1 - (threshold - delta) / threshold` clamped to 0..1, i.e. `delta / threshold`.
fn threshold_fraction(delta: f32, threshold: f32) -> f32 {
    if threshold.is_nan() || threshold <= 0.0 {
        return if delta > 0.0 { 1.0 } else { 0.0 };
    }
    (delta / threshold).clamp(0.0, 1.0)
}

fn blend(dec: &Levels, acc: &Levels, load: f32) -> Levels {
    Levels {
        aems: lerp(dec.aems, acc.aems, load),
        accel: lerp(dec.accel, acc.accel, load),
        decel: lerp(dec.decel, acc.decel, load),
        cutoff: lerp(dec.cutoff, acc.cutoff, load),
    }
}
