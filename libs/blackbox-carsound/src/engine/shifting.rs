//! The shift state machine: what the audio RPM, torque and volume do through a gear change. Spec:
//! `docs/specs/engine-sound-effects.md` §1.

use super::{EngineEvents, ShiftDirection, ShiftState, TickContext};
use crate::engine::graph::{Graph, fill_graph};
use crate::input::GEAR_NEUTRAL;
use crate::interp::{Curve, Interp};
use crate::tuning::ShiftTuning;

/// Up shifts are ignored below this physics RPM; down shifts make a clunk only above it.
const MIN_RPM_FOR_SHIFT_FX: f32 = 3000.0;
/// Second gear id: first gear and below get the shortened down-shift stages.
const SECOND_GEAR: i32 = 3;
/// The sixth gear id and up get no post-shift wobble.
const SIXTH_GEAR: i32 = 7;
/// RPM the tachometer value drops by over an up shift.
const VISUAL_DROP: f32 = 1200.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct Shifting {
    pub state: ShiftState,
    /// The stage entered this tick, or `None`.
    pub stage_changed: ShiftState,
    pub rpm_at_shift: f32,
    stage: usize,
    stage_ms: f32,
    graph: Graph,
    rpm_offset: f32,
    rpm: Interp,
    torque: Interp,
    volume: Interp,
    /// The tachometer's own value through an up shift.
    pub visual: Interp,
    rpm_decay: Interp,
    vol_decay: Interp,
    lfo_on: bool,
    pub lfo_rpm_amp: f32,
    pub lfo_rpm_period_ms: f32,
    pub lfo_vol_amp: f32,
    pub lfo_vol_period_ms: f32,
    pending_clunk: Option<(f32, ShiftDirection)>,
}

impl Shifting {
    pub fn new() -> Self {
        Self {
            state: ShiftState::None,
            stage_changed: ShiftState::None,
            rpm_at_shift: 0.0,
            stage: 0,
            stage_ms: 0.0,
            graph: Graph::default(),
            rpm_offset: 0.0,
            rpm: Interp::default(),
            torque: Interp::default(),
            volume: Interp::default(),
            visual: Interp::default(),
            rpm_decay: Interp::default(),
            vol_decay: Interp::default(),
            lfo_on: false,
            lfo_rpm_amp: 0.0,
            lfo_rpm_period_ms: 0.0,
            lfo_vol_amp: 0.0,
            lfo_vol_period_ms: 0.0,
            pending_clunk: None,
        }
    }

    pub fn active(&self) -> bool {
        self.state != ShiftState::None
    }

    pub fn shifting_rpm(&self) -> f32 {
        self.rpm.value()
    }

    pub fn shifting_torque(&self) -> f32 {
        self.torque.value()
    }

    pub fn shifting_volume(&self) -> f32 {
        self.volume.value()
    }

    /// Runs the triggers and one tick of the state machine.
    pub fn update(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning, events: &mut EngineEvents) {
        self.trigger(ctx, tuning, events);
        self.update_state(ctx, tuning, events);
    }

    fn trigger(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning, events: &mut EngineEvents) {
        let physics = ctx.physics;
        if physics.gear > physics.last_gear {
            self.begin_up(ctx, tuning, events);
            return;
        }
        if physics.gear < physics.last_gear {
            self.begin_down(ctx, tuning);
        }
    }

    fn begin_up(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning, events: &mut EngineEvents) {
        self.clean_up();
        if ctx.physics.rpm < MIN_RPM_FOR_SHIFT_FX {
            return;
        }
        let (Some(first), Some(engage)) = (tuning.up_disengage_fall.first(), tuning.up_engage) else { return };
        self.state = ShiftState::UpDisengage;
        self.stage_changed = ShiftState::UpDisengage;
        self.rpm_at_shift = ctx.eng_rpm;
        self.graph = fill_graph(first);
        self.stage = 0;
        self.stage_ms = 0.0;
        self.rpm_offset = self.rpm_at_shift.trunc();
        self.rpm.begin(self.rpm_at_shift, self.rpm_at_shift, 0.0, Curve::Linear);
        self.volume.begin(0.0, 0.0, 1.0, Curve::Linear);
        self.torque.begin(ctx.physics.torque, 0.0, 100.0, Curve::Linear);
        let mut total = first.time_ms as f32 + engage.time_ms as f32;
        if let Some(second) = tuning.up_disengage_fall.get(1) {
            total += second.time_ms as f32;
        }
        self.visual.begin(self.rpm_at_shift, self.rpm_at_shift - VISUAL_DROP, total, Curve::EqPowerSq);
        self.pending_clunk = Some((ctx.now + tuning.up_sound_delay, ShiftDirection::Up));
        events.disengage_sweetener = true;
    }

    fn begin_down(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning) {
        if ctx.physics.gear == GEAR_NEUTRAL {
            return;
        }
        self.clean_up();
        self.state = ShiftState::DownDisengage;
        self.stage_changed = ShiftState::DownDisengage;
        self.rpm_at_shift = ctx.eng_rpm;
        self.torque.begin(ctx.physics.torque, 0.0, 50.0, Curve::Linear);
        let (fall_rpm, fall_ms) = tuning.down_disengage_fall;
        let target = (self.rpm_at_shift - fall_rpm as f32).clamp(1000.0, 10000.0);
        self.rpm.begin(self.rpm_at_shift, target, fall_ms as f32, Curve::Linear);
        self.volume.begin(0.0, 0.0, 1.0, Curve::Linear);
        if ctx.physics.rpm > MIN_RPM_FOR_SHIFT_FX {
            self.pending_clunk = Some((ctx.now + tuning.down_sound_delay, ShiftDirection::Down));
        }
    }

    fn update_state(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning, events: &mut EngineEvents) {
        if let Some((at, direction)) = self.pending_clunk
            && ctx.now > at
        {
            events.gear_clunk = Some(direction);
            self.pending_clunk = None;
        }
        self.stage_changed = ShiftState::None;
        if self.state == ShiftState::None {
            return;
        }
        self.update_rpm(ctx);
        self.update_torque(ctx);
        self.volume.update(ctx.dt);
        self.post_shift_update(ctx.dt, tuning);
        self.stage_ms += ctx.dt * 1000.0;
        match self.state {
            ShiftState::UpDisengage => self.step_up_disengage(ctx, tuning, events),
            ShiftState::UpEngaging => self.step_up_engaging(ctx),
            ShiftState::UpLfo => self.step_up_lfo(),
            ShiftState::DownDisengage => self.step_down_disengage(ctx, tuning),
            ShiftState::DownEngagingRise => self.step_down_rise(ctx, tuning),
            ShiftState::DownEngagingFall => self.step_down_fall(ctx, tuning),
            ShiftState::DownEngagingReattach => self.step_down_reattach(),
            ShiftState::None => {}
        }
    }

    fn update_rpm(&mut self, ctx: &TickContext<'_>) {
        if !self.visual.finished() {
            self.visual.update_live(ctx.dt, ctx.physics.rpm);
        }
        match self.state {
            ShiftState::UpLfo | ShiftState::DownEngagingReattach => self.rpm.update_live(ctx.dt, ctx.physics.rpm),
            ShiftState::None => {}
            _ => self.rpm.update(ctx.dt),
        }
    }

    fn update_torque(&mut self, ctx: &TickContext<'_>) {
        match self.state {
            ShiftState::UpEngaging | ShiftState::UpLfo | ShiftState::DownEngagingReattach => {
                self.torque.update_live(ctx.dt, ctx.physics.torque)
            }
            ShiftState::None => {}
            _ => self.torque.update(ctx.dt),
        }
    }

    fn hold_rpm(&mut self, offset: f32) {
        let now = (offset + self.graph.value(self.stage_ms)).clamp(1000.0, 10000.0);
        self.rpm.begin(now, now, 0.0, Curve::Linear);
    }

    fn step_up_disengage(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning, events: &mut EngineEvents) {
        if self.stage_ms <= self.graph.end_ms() {
            self.hold_rpm(self.rpm_offset);
            return;
        }
        self.stage += 1;
        if let Some(next) = tuning.up_disengage_fall.get(self.stage) {
            self.graph = fill_graph(next);
            self.stage_ms = ctx.dt * 1000.0;
            self.rpm_offset = self.rpm.value().trunc();
            self.hold_rpm(self.rpm_offset);
            return;
        }
        let Some(engage) = tuning.up_engage else {
            self.clean_up();
            return;
        };
        self.state = ShiftState::UpEngaging;
        self.stage_changed = ShiftState::UpEngaging;
        self.graph = fill_graph(&engage);
        self.stage_ms = 0.0;
        self.rpm_offset = ctx.physics.rpm.trunc();
        self.hold_rpm(self.rpm_offset);
        self.torque.begin(ctx.physics.torque, ctx.physics.torque, 0.0, Curve::Linear);
        self.volume.begin(tuning.up_engage_attack_volume, 0.0, tuning.up_engage_attack_ms as f32, Curve::Linear);
        self.post_shift_init(ctx, tuning);
        self.post_shift_update(ctx.dt, tuning);
        events.engage_sweetener = true;
    }

    fn step_up_engaging(&mut self, ctx: &TickContext<'_>) {
        if self.stage_ms > self.graph.end_ms() {
            self.state = ShiftState::UpLfo;
            self.stage_changed = ShiftState::UpLfo;
            return;
        }
        self.rpm_offset = ctx.physics.rpm.trunc();
        self.hold_rpm(self.rpm_offset);
    }

    fn step_up_lfo(&mut self) {
        let running = !self.rpm_decay.finished() || !self.vol_decay.finished();
        if running && self.lfo_on {
            return;
        }
        self.clean_up();
    }

    fn step_down_disengage(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning) {
        if !self.rpm.finished() {
            return;
        }
        self.state = ShiftState::DownEngagingRise;
        self.stage_changed = ShiftState::DownEngagingRise;
        let (rise_rpm, rise_ms) = tuning.down_engage_rise;
        let mut length = rise_ms as f32;
        if ctx.physics.gear < SECOND_GEAR {
            length = (length * 0.7).trunc();
        }
        let low = low_rpm_scale(ctx.eng_rpm);
        let target = (ctx.physics.rpm + rise_rpm as f32 * low).clamp(1000.0, 10000.0);
        self.rpm.begin(ctx.eng_rpm, target, length, Curve::EqPowerSq);
        self.torque.begin(ctx.eng_torque, REV_PERCENT * low, REV_RAMP_MS, Curve::Linear);
    }

    fn step_down_rise(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning) {
        if !self.rpm.finished() {
            return;
        }
        self.state = ShiftState::DownEngagingFall;
        self.stage_changed = ShiftState::DownEngagingFall;
        let (fall_rpm, fall_ms) = tuning.down_engage_fall;
        let mut length = fall_ms as f32;
        if ctx.physics.gear < SECOND_GEAR && tuning.upgrade_level <= 1 {
            length = (length * 0.7).trunc();
        }
        let low = low_rpm_scale(ctx.eng_rpm);
        self.rpm.begin(ctx.eng_rpm, ctx.eng_rpm - fall_rpm as f32 * low, length, Curve::Linear);
        self.torque.begin(REV_PERCENT * low, 0.0, REV_RAMP_MS, Curve::Linear);
    }

    fn step_down_fall(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning) {
        if !self.rpm.finished() {
            return;
        }
        self.state = ShiftState::DownEngagingReattach;
        self.stage_changed = ShiftState::DownEngagingReattach;
        let attach_ms = ((ctx.eng_rpm - ctx.physics.rpm).abs() * tuning.down_reattach_scale).clamp(0.0, 800.0);
        self.rpm.begin(ctx.eng_rpm, ctx.physics.rpm, attach_ms.trunc(), Curve::EqPowerSq);
        self.torque.begin(0.0, ctx.physics.torque, 60.0, Curve::Linear);
    }

    fn step_down_reattach(&mut self) {
        if self.rpm.finished() {
            self.clean_up();
        }
    }

    fn post_shift_init(&mut self, ctx: &TickContext<'_>, tuning: &ShiftTuning) {
        if ctx.physics.gear >= SIXTH_GEAR {
            return;
        }
        self.lfo_on = true;
        self.rpm_decay.begin(
            tuning.lfo_rpm.amplitude as f32 * gear_lfo_scale(ctx.physics.gear),
            0.0,
            tuning.lfo_rpm.decay_ms as f32,
            Curve::Linear,
        );
        self.vol_decay.begin(tuning.lfo_volume.amplitude as f32, 0.0, tuning.lfo_volume.decay_ms as f32, Curve::Linear);
    }

    fn post_shift_update(&mut self, dt: f32, tuning: &ShiftTuning) {
        if !self.lfo_on {
            return;
        }
        self.rpm_decay.update(dt);
        self.vol_decay.update(dt);
        if self.rpm_decay.finished() {
            self.clean_up();
            return;
        }
        self.lfo_rpm_amp = self.rpm_decay.value();
        self.lfo_rpm_period_ms = tuning.lfo_rpm.period_ms as f32;
        self.lfo_vol_amp = self.vol_decay.value();
        self.lfo_vol_period_ms = tuning.lfo_volume.period_ms as f32;
    }

    fn clean_up(&mut self) {
        self.lfo_on = false;
        self.lfo_rpm_amp = 0.0;
        self.lfo_rpm_period_ms = 0.0;
        self.lfo_vol_amp = 0.0;
        self.lfo_vol_period_ms = 0.0;
        self.state = ShiftState::None;
        self.stage_changed = ShiftState::None;
    }
}

/// The torque (percent) of the rev-matching blip and the time its ramps take.
const REV_PERCENT: f32 = 100.0;
const REV_RAMP_MS: f32 = 215.0;

/// No blip when the engine is at idle.
fn low_rpm_scale(eng_rpm: f32) -> f32 {
    if eng_rpm < 1500.0 { 0.0 } else { 1.0 }
}

/// How much of the post-shift wobble each gear keeps (first gear 1, second 0.85, ... fifth 0.3).
fn gear_lfo_scale(gear: i32) -> f32 {
    match gear {
        3 => 0.85,
        4 => 0.7,
        5 => 0.55,
        6 => 0.3,
        _ => 1.0,
    }
}
