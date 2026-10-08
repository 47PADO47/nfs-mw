//! The engine controller: the audio RPM (`EngRPM`), torque, volume factor, the post-shift wobble, the
//! cruising compression bump, the redline and the tachometer value. Spec: `docs/specs/engine-sound.md` §4 and §7.

use super::accel_trans::{AccelState, AccelTrans};
use super::physics::Physics;
use super::shifting::Shifting;
use super::{ShiftState, TickContext};
use crate::avg::RunningAverage;
use crate::interp::{Curve, Interp};
use crate::math::{equal_power_sq, finite, slew, slew_asym};
use crate::rng::Rng;
use crate::tuning::EngineTuning;

/// Above this audio RPM the clutch model is off.
const CLUTCH_RPM_THRESHOLD: f32 = 2500.0;
/// The clutch model: `EngRPM` follows the physics at most this much up and down per tick.
const CLUTCH_UP: f32 = 999.0;
const CLUTCH_DOWN: f32 = 60.0;
/// Audio RPM above which the engine ducks (sound scale).
const REDLINE_RPM: f32 = 9800.0;
/// The share of the mix the redline sample takes.
const REDLINE_MIX: f32 = 0.85;
/// Redline fade lengths in ms: engine out and in, sample out and in.
const ENGINE_FADE_OUT_MS: f32 = 450.0;
const ENGINE_FADE_IN_MS: f32 = 50.0;
const SAMPLE_FADE_OUT_MS: f32 = 50.0;
const SAMPLE_FADE_IN_MS: f32 = 120.0;
/// Height of the tachometer bounce at the limiter and its speed per tick.
const BOUNCE_HEIGHT: f32 = 200.0;
const BOUNCE_STEP: f32 = 50.0;
/// The tachometer blends from the physics value to the audio value over this long after the countdown.
const MERGE_SECONDS: f32 = 0.7;
/// Where the wobble's phase starts (about a quarter turn: its peak).
const LFO_START_PHASE: f32 = 16535.0 / 65536.0;

/// A short rise and fall of the audio RPM played at random in steady cruising.
#[derive(Debug, Clone, Copy, Default)]
struct Bump {
    height: f32,
    seconds: f32,
    elapsed: f32,
}

impl Bump {
    fn value(&self) -> f32 {
        if self.seconds <= 0.0 {
            return 0.0;
        }
        if self.elapsed >= 2.0 * self.seconds {
            return 0.0;
        }
        if self.elapsed < self.seconds {
            return self.height * equal_power_sq(self.elapsed / self.seconds);
        }
        self.height * (1.0 - equal_power_sq((self.elapsed - self.seconds) / self.seconds))
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EngineCtl {
    /// `EngRPM` and the value before it, on the sound scale.
    pub rpm: f32,
    pub prev_rpm: f32,
    /// `EngTorque`: the 3-tick average of the torque in percent.
    pub torque: f32,
    /// `EngVolume`: 32767 is unity.
    pub volume: f32,
    pub redlining: bool,
    pub was_redlining: bool,
    pub eng_factor: Interp,
    pub samp_factor: Interp,
    /// The tachometer value as a fraction of the gauge.
    pub visual_fraction: f32,
    /// Set by the hybrid motor to start a compression bump.
    pub play_compression: bool,
    /// A compression bump started on the latest tick (the engine control publishes it to the mixer).
    pub bump_started: bool,
    torque_avg: RunningAverage<3>,
    visual_avg: RunningAverage<2>,
    clutch_on: bool,
    rpm_lfo: f32,
    vol_lfo: f32,
    rpm_phase: f32,
    vol_phase: f32,
    bump: Bump,
    bounce_up: bool,
    visual_offset: f32,
    pre_race: bool,
    merge_timer: f32,
}

impl EngineCtl {
    pub fn new() -> Self {
        Self {
            rpm: 1000.0,
            prev_rpm: 1000.0,
            torque: 0.0,
            volume: 32767.0,
            redlining: false,
            was_redlining: false,
            eng_factor: Interp::holding(1.0),
            samp_factor: Interp::holding(0.0),
            visual_fraction: 0.0,
            play_compression: false,
            bump_started: false,
            torque_avg: RunningAverage::flushed(0.0),
            visual_avg: RunningAverage::flushed(1000.0),
            clutch_on: false,
            rpm_lfo: 0.0,
            vol_lfo: 0.0,
            rpm_phase: LFO_START_PHASE,
            vol_phase: LFO_START_PHASE,
            bump: Bump::default(),
            bounce_up: true,
            visual_offset: 0.0,
            pre_race: false,
            merge_timer: 0.0,
        }
    }

    fn set_rpm(&mut self, rpm: f32) {
        self.prev_rpm = self.rpm;
        self.rpm = finite(rpm);
    }

    /// The engine controller's part of one tick.
    pub fn update(
        &mut self,
        ctx: &TickContext<'_>,
        shift: &Shifting,
        accel: &AccelTrans,
        tuning: &EngineTuning,
        rng: &mut Rng,
    ) {
        self.clutch_on = ctx.is_local_player && !accel.active() && self.rpm <= CLUTCH_RPM_THRESHOLD;
        self.update_lfo(ctx.dt, shift);
        self.update_bump(ctx.dt, rng);
        self.update_rpm(ctx, shift, accel);
        self.update_torque(ctx.physics, shift, accel);
        self.update_volume(shift);
        self.update_redline(ctx, shift, tuning);
    }

    fn update_lfo(&mut self, dt: f32, shift: &Shifting) {
        if !shift.active() {
            self.rpm_lfo = 0.0;
            self.vol_lfo = 0.0;
            self.rpm_phase = LFO_START_PHASE;
            self.vol_phase = LFO_START_PHASE;
            return;
        }
        if shift.lfo_rpm_amp != 0.0 {
            self.rpm_phase = advance_phase(self.rpm_phase, dt, shift.lfo_rpm_period_ms);
            self.rpm_lfo = shift.lfo_rpm_amp * (self.rpm_phase * std::f32::consts::TAU).sin();
        }
        if shift.lfo_vol_amp != 0.0 {
            self.vol_phase = advance_phase(self.vol_phase, dt, shift.lfo_vol_period_ms);
            self.vol_lfo = shift.lfo_vol_amp * (self.vol_phase * std::f32::consts::TAU).sin();
        }
    }

    fn update_bump(&mut self, dt: f32, rng: &mut Rng) {
        self.bump.elapsed += dt;
        self.bump_started = false;
        if !self.play_compression {
            return;
        }
        self.play_compression = false;
        self.bump_started = true;
        self.bump =
            Bump { height: 25.0 + rng.below(75) as f32, seconds: (25 + rng.below(100)) as f32 / 1000.0, elapsed: 0.0 };
    }

    fn update_rpm(&mut self, ctx: &TickContext<'_>, shift: &Shifting, accel: &AccelTrans) {
        let physics = ctx.physics;
        let mut cur = physics.rpm;
        if shift.active() {
            cur = shift.shifting_rpm();
        } else if accel.active() {
            cur = accel.rpm();
        }
        if self.clutch_on && !shift.active() {
            cur = slew_asym(self.rpm, physics.rpm, CLUTCH_UP, CLUTCH_DOWN);
        }
        self.set_rpm(cur + self.rpm_lfo + self.bump.value() + self.rpm_lfo);

        let mut visual = cur;
        if matches!(shift.state, ShiftState::UpDisengage | ShiftState::UpEngaging) {
            visual = shift.visual.value();
        } else if accel.state == AccelState::Attack {
            visual = physics.rpm;
        } else if self.redlining {
            visual -= self.bounce_offset();
        }
        self.visual_avg.record(visual);
        let mut fraction = (self.visual_avg.value() - 1000.0) / 9000.0;
        fraction = self.blend_with_physics(ctx, fraction);
        self.visual_fraction = finite(fraction).clamp(0.0, 1.0);
    }

    /// The tachometer shows the physics value before the race and blends to the audio value after it.
    fn blend_with_physics(&mut self, ctx: &TickContext<'_>, audio: f32) -> f32 {
        let physics = ctx.physics.rpm_pct;
        if self.pre_race && !ctx.pre_race {
            self.merge_timer = MERGE_SECONDS;
        }
        self.pre_race = ctx.pre_race;
        if !ctx.is_local_player {
            return audio;
        }
        if ctx.pre_race {
            return physics;
        }
        if self.merge_timer <= 0.0 {
            return audio;
        }
        self.merge_timer = (self.merge_timer - ctx.dt).max(0.0);
        let blend = (MERGE_SECONDS - self.merge_timer) / MERGE_SECONDS;
        (audio - physics) * blend + physics
    }

    fn bounce_offset(&mut self) -> f32 {
        let target = if self.bounce_up { BOUNCE_HEIGHT } else { 0.0 };
        self.visual_offset = slew(self.visual_offset, target, BOUNCE_STEP);
        if self.visual_offset == target {
            self.bounce_up = !self.bounce_up;
        }
        self.visual_offset
    }

    fn update_torque(&mut self, physics: &Physics, shift: &Shifting, accel: &AccelTrans) {
        if shift.active() {
            self.torque_avg.flush(shift.shifting_torque());
        } else if accel.active() {
            self.torque_avg.flush(accel.torque());
        } else {
            self.torque_avg.record(physics.torque);
        }
        self.torque = self.torque_avg.value();
    }

    fn update_volume(&mut self, shift: &Shifting) {
        let mut volume = 32767.0;
        if shift.active() {
            volume += 32767.0 * shift.shifting_volume();
        }
        self.volume = finite(volume + self.vol_lfo);
    }

    fn update_redline(&mut self, ctx: &TickContext<'_>, shift: &Shifting, tuning: &EngineTuning) {
        self.was_redlining = self.redlining;
        if !tuning.redline_enabled {
            return;
        }
        if !self.redlining && !shift.active() && self.rpm > REDLINE_RPM {
            self.begin_redline((ctx.physics.gear as f32).clamp(1.0, 5.0));
        } else if self.redlining && (self.rpm < REDLINE_RPM || shift.active()) {
            self.end_redline();
        }
        self.eng_factor.update(ctx.dt);
        self.samp_factor.update(ctx.dt);
    }

    /// The engine ducks to 15 % and the sample takes over at 85 %; both fades are longer in higher gears.
    fn begin_redline(&mut self, gear_scale: f32) {
        self.redlining = true;
        let eng = self.eng_factor.value();
        let samp = self.samp_factor.value();
        let duck_ms = (ENGINE_FADE_OUT_MS * eng * gear_scale).trunc();
        let raise_ms = (SAMPLE_FADE_IN_MS * (1.0 - samp) * gear_scale).trunc();
        self.eng_factor.begin(eng, 1.0 - REDLINE_MIX, duck_ms, Curve::Linear);
        self.samp_factor.begin(samp, REDLINE_MIX, raise_ms, Curve::Linear);
        self.bounce_up = true;
        self.visual_offset = 0.0;
    }

    fn end_redline(&mut self) {
        self.redlining = false;
        let eng = self.eng_factor.value();
        let samp = self.samp_factor.value();
        self.eng_factor.begin(eng, 1.0, (ENGINE_FADE_IN_MS * (1.0 - eng)).trunc(), Curve::Linear);
        self.samp_factor.begin(samp, 0.0, (SAMPLE_FADE_OUT_MS * samp).trunc(), Curve::Linear);
    }
}

/// Advances a wobble phase (in turns) by `dt` seconds of a wobble with the given period (1 ms to 10 s).
fn advance_phase(phase: f32, dt: f32, period_ms: f32) -> f32 {
    let period = finite(period_ms).clamp(1.0, 10000.0) / 1000.0;
    (phase + dt / period).fract()
}
