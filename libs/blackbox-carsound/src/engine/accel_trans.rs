//! The throttle-stab rev: a short RPM move when the pedal is pressed. Spec:
//! `docs/specs/engine-sound-effects.md` §2.

use super::physics::Physics;
use super::{EngineEvents, TickContext};
use crate::input::GEAR_FIRST;
use crate::interp::{Curve, Interp};
use crate::tuning::AccelTransition;

/// A throttle rise (percentage points between two ticks) that counts as a stab.
const THROTTLE_SENSITIVITY: f32 = 30.0;
/// The attack: RPM above the physics value at the start and its length.
const ATTACK_RPM: f32 = 1000.0;
const ATTACK_MS: f32 = 500.0;
/// Minimum time between two attacks.
const ATTACK_GAP: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum AccelState {
    #[default]
    None,
    Attack,
    IdleReving,
    IdleEngaging,
    Interrupt,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AccelTrans {
    pub state: AccelState,
    rpm: Interp,
    torque: Interp,
    last_transition: f32,
    accelerating: bool,
}

impl AccelTrans {
    pub fn new() -> Self {
        Self {
            state: AccelState::None,
            rpm: Interp::default(),
            torque: Interp::default(),
            last_transition: 0.0,
            accelerating: false,
        }
    }

    pub fn active(&self) -> bool {
        self.state != AccelState::None
    }

    pub fn rpm(&self) -> f32 {
        self.rpm.value()
    }

    pub fn torque(&self) -> f32 {
        self.torque.value()
    }

    pub fn update(
        &mut self,
        ctx: &TickContext<'_>,
        shift_active: bool,
        tuning: &AccelTransition,
        events: &mut EngineEvents,
    ) {
        let (dt, now, physics, speed_mph, eng_rpm) = (ctx.dt, ctx.now, ctx.physics, ctx.speed_mph, ctx.eng_rpm);
        let was_accelerating = self.accelerating;
        self.accelerating = physics.accelerating;
        if self.accelerating && !was_accelerating {
            if self.should_begin_idle(physics, speed_mph, shift_active) {
                self.begin_idle(now, physics, tuning);
            } else if self.should_begin_attack(now, physics, shift_active) {
                self.begin_attack(now, physics, events);
            }
        }
        if !self.accelerating && was_accelerating && self.should_play_engine_off(physics, shift_active) {
            events.engine_off_sweetener = true;
        }
        self.update_state(dt, physics, eng_rpm, tuning);
    }

    fn rose_fast(physics: &Physics) -> bool {
        physics.throttle - physics.old_throttle >= THROTTLE_SENSITIVITY
    }

    fn should_begin_idle(&self, physics: &Physics, speed_mph: f32, shift_active: bool) -> bool {
        speed_mph <= 15.0
            && Self::rose_fast(physics)
            && self.state == AccelState::None
            && !shift_active
            && physics.gear == GEAR_FIRST
            && physics.rpm <= 1500.0
    }

    fn should_begin_attack(&self, now: f32, physics: &Physics, shift_active: bool) -> bool {
        Self::rose_fast(physics)
            && self.state == AccelState::None
            && !shift_active
            && physics.rpm >= 3000.0
            && now - self.last_transition >= ATTACK_GAP
            && physics.gear > GEAR_FIRST
    }

    fn should_play_engine_off(&self, physics: &Physics, shift_active: bool) -> bool {
        self.state == AccelState::None && !shift_active && physics.rpm >= 6000.0 && physics.gear > GEAR_FIRST
    }

    fn begin_attack(&mut self, now: f32, physics: &Physics, events: &mut EngineEvents) {
        self.rpm.begin(physics.rpm + ATTACK_RPM, physics.rpm, ATTACK_MS, Curve::EqPowerSq);
        self.state = AccelState::Attack;
        self.torque.begin(100.0, 100.0, 10.0, Curve::Linear);
        self.last_transition = now;
        events.accel_sweetener = true;
    }

    fn begin_idle(&mut self, now: f32, physics: &Physics, tuning: &AccelTransition) {
        self.state = AccelState::IdleReving;
        self.last_transition = now;
        self.rpm.begin(physics.rpm, physics.rpm + tuning.peak_rpm as f32, tuning.peak_ms as f32, Curve::Linear);
        self.torque.begin(physics.torque, physics.torque, 10.0, Curve::Linear);
    }

    fn update_state(&mut self, dt: f32, physics: &Physics, eng_rpm: f32, tuning: &AccelTransition) {
        self.update_rpm(dt, physics);
        self.update_torque(dt, physics);
        if self.state == AccelState::None {
            return;
        }
        if !self.accelerating && self.state != AccelState::Interrupt {
            self.rpm.begin(eng_rpm, physics.rpm, tuning.interrupt_ms as f32, Curve::EqPowerSq);
            self.state = AccelState::Interrupt;
        }
        match self.state {
            AccelState::IdleReving => {
                if self.rpm.finished() {
                    self.state = AccelState::IdleEngaging;
                    self.rpm.begin(self.rpm.value(), physics.rpm, tuning.resume_ms as f32, Curve::Linear);
                }
            }
            AccelState::Interrupt | AccelState::Attack | AccelState::IdleEngaging => {
                if self.rpm.finished() {
                    self.state = AccelState::None;
                }
            }
            AccelState::None => {}
        }
    }

    fn update_rpm(&mut self, dt: f32, physics: &Physics) {
        match self.state {
            AccelState::None => {}
            AccelState::IdleReving => self.rpm.update(dt),
            _ => self.rpm.update_live(dt, physics.rpm),
        }
    }

    fn update_torque(&mut self, dt: f32, physics: &Physics) {
        if self.state == AccelState::None {
            return;
        }
        self.torque.update_live(dt, physics.torque);
        if self.torque.finished() {
            self.torque.begin(physics.torque, physics.torque, 0.0, Curve::Linear);
        }
    }
}
