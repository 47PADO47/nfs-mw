//! The engine sound: the audio RPM, the shift and throttle-stab behaviour, the layer volumes and the Ginsu
//! drive values. Specs: `docs/specs/engine-sound.md` and `docs/specs/engine-sound-effects.md` §1 and §2.
//!
//! The controllers of the original run once per frame with constants that only make sense at a fixed rate;
//! here they run on a fixed 60 Hz tick ([`EngineMixer::update`] accumulates the frame time).

mod accel_trans;
mod graph;
mod mix;
mod physics;
mod rpm;
mod shifting;

use self::accel_trans::AccelTrans;
use self::mix::HybridMix;
use self::physics::Physics;
use self::rpm::EngineCtl;
use self::shifting::Shifting;
use crate::input::CarInput;
use crate::math::{finite, q15_gain};
use crate::rng::Rng;
use crate::tuning::{AccelTransition, CarSoundTuning, EngineTuning, ShiftTuning};

/// The controllers' tick rate.
pub const TICK_HZ: f32 = 60.0;
/// Length of one tick, seconds.
pub const TICK_SECONDS: f32 = 1.0 / TICK_HZ;
/// Most ticks one update runs; the time beyond that is dropped so a stall cannot cause a burst.
pub const MAX_TICKS_PER_UPDATE: u32 = 10;

/// The stage of a gear shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShiftState {
    #[default]
    None,
    UpDisengage,
    UpEngaging,
    UpLfo,
    DownDisengage,
    DownEngagingRise,
    DownEngagingFall,
    DownEngagingReattach,
}

/// Direction of a gear change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftDirection {
    Up,
    Down,
}

/// One-shot happenings of the engine side that the effects turn into sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EngineEvents {
    /// The gear clunk is due.
    pub gear_clunk: Option<ShiftDirection>,
    /// An up shift began (the sweetener plays only at high RPM).
    pub disengage_sweetener: bool,
    /// The up shift's engage stage began.
    pub engage_sweetener: bool,
    /// A throttle stab began its attack.
    pub accel_sweetener: bool,
    /// The throttle was released at high RPM.
    pub engine_off_sweetener: bool,
}

impl EngineEvents {
    /// Adds the events of `other` (a later tick) to these.
    pub fn merge(&mut self, other: &EngineEvents) {
        self.gear_clunk = other.gear_clunk.or(self.gear_clunk);
        self.disengage_sweetener |= other.disengage_sweetener;
        self.engage_sweetener |= other.engage_sweetener;
        self.accel_sweetener |= other.accel_sweetener;
        self.engine_off_sweetener |= other.engine_off_sweetener;
    }
}

/// What to give the Ginsu synthesiser of one loop: the frequency to track and the playback-rate ratio.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LoopDrive {
    /// Target frequency in the file's units, not below the loop's minimum when that is known.
    pub frequency: f32,
    /// Resampling ratio at the player: `pitch1 * pitch multiplier` (1 = unchanged).
    pub playback_rate: f32,
}

impl LoopDrive {
    /// Below `min_frequency` the loop plays at its minimum and the playback rate carries the ratio (spec §6).
    pub fn new(frequency: f32, min_frequency: f32, pitch_multiplier: f32) -> Self {
        if min_frequency > 0.0 && frequency < min_frequency {
            return Self { frequency: min_frequency, playback_rate: frequency / min_frequency * pitch_multiplier };
        }
        Self { frequency, playback_rate: pitch_multiplier }
    }
}

/// The engine sound of the latest tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EngineOutput {
    /// The Ginsu target frequency (before any per-loop minimum).
    pub ginsu_frequency: f32,
    /// Playback-rate ratio of the accelerate loop (`pitch1 * pitch multiplier`).
    pub playback_rate: f32,
    pub accel_loop: LoopDrive,
    pub decel_loop: LoopDrive,
    /// Linear volumes, 0 to 1.
    pub accel_volume: f32,
    pub decel_volume: f32,
    /// Volume of the sample (AEMS) layer, which is not rendered yet.
    pub aems_volume: f32,
    /// Volume of the sample layer's redline sound.
    pub redline_sample_volume: f32,
    /// Low-pass cutoff for the loops, Hz.
    pub low_pass_hz: f32,
    /// The tachometer value as a fraction of idle..redline, to hand back to the physics.
    pub visual_rpm_fraction: f32,
    /// `TORQUE` for the sample layer, 0 to 1024.
    pub aems_torque: f32,
    /// Audio RPM and torque (percent) and the physics RPM, on the 1000 to 10000 sound scale.
    pub eng_rpm: f32,
    pub eng_torque: f32,
    pub physics_rpm: f32,
    /// The throttle in percent.
    pub throttle: f32,
    pub accelerating: bool,
    pub redlining: bool,
    pub shift_state: ShiftState,
    /// The `EngRPM` at the start of the latest shift.
    pub rpm_at_shift: f32,
    /// Happenings since the previous [`EngineMixer::update`] call.
    pub events: EngineEvents,
}

/// What the controllers of one tick share.
pub(crate) struct TickContext<'a> {
    pub dt: f32,
    pub now: f32,
    pub physics: &'a Physics,
    pub speed_mph: f32,
    /// `EngRPM` and `EngTorque` as the engine left them last tick.
    pub eng_rpm: f32,
    pub eng_torque: f32,
    pub is_local_player: bool,
    pub pre_race: bool,
}

/// The engine controllers of one car, ticked at the fixed rate.
#[derive(Debug, Clone)]
pub(crate) struct EngineCore {
    tuning: EngineTuning,
    shift_tuning: ShiftTuning,
    accel_tuning: AccelTransition,
    is_local_player: bool,
    rng: Rng,
    time: f32,
    pitch_multiplier: f32,
    physics: Physics,
    shifting: Shifting,
    accel: AccelTrans,
    ctl: EngineCtl,
    mix: HybridMix,
}

impl EngineCore {
    pub fn new(tuning: &CarSoundTuning) -> Self {
        Self {
            tuning: tuning.engine.clone(),
            shift_tuning: tuning.shift.clone(),
            accel_tuning: tuning.accel_transition,
            is_local_player: tuning.is_local_player,
            rng: Rng::new(tuning.seed),
            time: 0.0,
            pitch_multiplier: 1.0,
            physics: Physics::new(),
            shifting: Shifting::new(),
            accel: AccelTrans::new(),
            ctl: EngineCtl::new(),
            mix: HybridMix::new(),
        }
    }

    /// Runs one tick; the happenings of the tick are added to `events`.
    pub fn tick(&mut self, input: &CarInput, events: &mut EngineEvents) {
        self.time += TICK_SECONDS;
        let pitch = finite(input.pitch_multiplier);
        self.pitch_multiplier = if pitch > 0.0 { pitch } else { 1.0 };
        self.physics.update(input, &self.tuning, self.is_local_player);
        let ctx = TickContext {
            dt: TICK_SECONDS,
            now: self.time,
            physics: &self.physics,
            speed_mph: input.speed_mph(),
            eng_rpm: self.ctl.rpm,
            eng_torque: self.ctl.torque,
            is_local_player: self.is_local_player,
            pre_race: input.pre_race,
        };
        self.shifting.update(&ctx, &self.shift_tuning, events);
        self.accel.update(&ctx, self.shifting.active(), &self.accel_tuning, events);
        self.ctl.update(&ctx, &self.shifting, &self.accel, &self.tuning, &mut self.rng);
        self.mix.update(&ctx, &self.ctl, &self.shifting, &self.accel, &self.tuning);
        self.mix.update_cruise(&ctx, &mut self.ctl, &mut self.rng);
    }

    /// The output of the latest tick, with `events` as its events.
    pub fn output(&self, events: EngineEvents) -> EngineOutput {
        let mix = &self.mix.output;
        let tuning = &self.tuning;
        let accel_loop = LoopDrive::new(mix.ginsu_frequency, tuning.accel_loop_min_frequency, self.pitch_multiplier);
        // The original derives the frequency and the playback ratio once, from the accelerate loop's lowest
        // frequency, and hands the same two values to the decelerate loop (spec: engine-sound.md §6).
        EngineOutput {
            ginsu_frequency: mix.ginsu_frequency,
            playback_rate: accel_loop.playback_rate,
            accel_loop,
            decel_loop: accel_loop,
            accel_volume: q15_gain(mix.accel_volume),
            decel_volume: q15_gain(mix.decel_volume),
            aems_volume: q15_gain(mix.aems_volume),
            redline_sample_volume: q15_gain(mix.redline_volume),
            low_pass_hz: mix.low_pass_hz,
            visual_rpm_fraction: self.ctl.visual_fraction,
            aems_torque: self.ctl.torque * 10.24,
            eng_rpm: self.ctl.rpm,
            eng_torque: self.ctl.torque,
            physics_rpm: self.physics.rpm,
            throttle: self.physics.throttle,
            accelerating: self.physics.accelerating,
            redlining: self.ctl.redlining,
            shift_state: self.shifting.state,
            rpm_at_shift: self.shifting.rpm_at_shift,
            events,
        }
    }
}

/// Turns frame times into whole ticks.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Ticker {
    accumulated: f32,
}

impl Ticker {
    /// Adds `dt` seconds and returns how many ticks to run (at most [`MAX_TICKS_PER_UPDATE`]).
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.accumulated += finite(dt).max(0.0);
        let whole = (self.accumulated / TICK_SECONDS + 1e-3).floor();
        if whole > MAX_TICKS_PER_UPDATE as f32 {
            self.accumulated = 0.0;
            return MAX_TICKS_PER_UPDATE;
        }
        let ticks = whole as u32;
        self.accumulated = (self.accumulated - ticks as f32 * TICK_SECONDS).max(0.0);
        ticks
    }
}

/// The engine mix of one car: telemetry in, Ginsu drive values and layer volumes out.
///
/// [`update`](Self::update) takes real frame times and runs the controllers on a fixed 60 Hz tick.
#[derive(Debug, Clone)]
pub struct EngineMixer {
    core: EngineCore,
    ticker: Ticker,
    output: EngineOutput,
}

impl EngineMixer {
    pub fn new(tuning: &CarSoundTuning) -> Self {
        let core = EngineCore::new(tuning);
        let output = core.output(EngineEvents::default());
        Self { core, ticker: Ticker::default(), output }
    }

    /// Advances by `dt` seconds with the latest telemetry and returns the latest output. The output keeps its
    /// value on frames that complete no tick; its `events` are those of this call only.
    pub fn update(&mut self, dt: f32, input: &CarInput) -> EngineOutput {
        let mut events = EngineEvents::default();
        for _ in 0..self.ticker.advance(dt) {
            self.core.tick(input, &mut events);
        }
        self.output = self.core.output(events);
        self.output
    }

    /// The output of the latest update.
    pub fn output(&self) -> &EngineOutput {
        &self.output
    }

    /// The tachometer value (a fraction of idle..redline) to hand back to the physics.
    pub fn visual_rpm_fraction(&self) -> f32 {
        self.output.visual_rpm_fraction
    }
}
