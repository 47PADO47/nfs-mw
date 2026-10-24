//! Global Speedbreaker clock ramp, integrated over real time without changing the fixed tick.

use super::Drive;
use crate::input::{Action, ActionState};

#[derive(Debug, Clone, Copy)]
pub(in crate::scenes::world) struct SpeedbreakerClock {
    multiplier: f64,
    dilation: crate::settings::speedbreaker::Dilation,
}

impl Default for SpeedbreakerClock {
    fn default() -> Self {
        Self { multiplier: 1.0, dilation: crate::settings::speedbreaker::Dilation::default() }
    }
}

impl SpeedbreakerClock {
    /// The entry target and entry/exit rates. Set from the world settings each frame.
    pub fn set_dilation(&mut self, dilation: crate::settings::speedbreaker::Dilation) {
        self.dilation = dilation;
    }

    /// Simulation seconds over this frame, including the linear entry/exit ramp.
    pub fn advance(&mut self, active: bool, real_dt: f32) -> f32 {
        if !real_dt.is_finite() || real_dt <= 0.0 {
            return 0.0;
        }
        let (target, rate) = match active {
            true => (f64::from(self.dilation.target), f64::from(self.dilation.enter_rate)),
            false => (1.0, f64::from(self.dilation.exit_rate)),
        };
        let dt = f64::from(real_dt);
        let ramp = dt.min((target - self.multiplier).abs() / rate);
        let before = self.multiplier;
        let change = rate * ramp;
        self.multiplier += (target - self.multiplier).clamp(-change, change);
        (((before + self.multiplier) * 0.5) * ramp + target * (dt - ramp)) as f32
    }
}

impl Drive {
    /// The Speedbreaker tunables the meter and the physics read. Set from the world settings each frame.
    pub fn set_speedbreaker_tuning(&mut self, tuning: crate::settings::SpeedbreakerTuning) {
        self.breaker.set_tuning(tuning);
    }

    /// Bars the speedbreaker while a session's rules forbid time scaling (`session_rules.rs`); a barred breaker is off.
    pub fn bar_speedbreaker(&mut self, barred: bool) {
        self.breaker_barred = barred;
    }

    /// Meter/button edges run every loaded frame, including frames with no 60 Hz tick.
    pub fn prepare_time(&mut self) -> bool {
        if self.arrested || self.breaker_barred || self.cinematic_input.is_some() {
            self.breaker.cancel();
            return false;
        }
        if self.request.is_some() || self.sim.is_none() || (self.batch_run && self.script_finished()) {
            return false;
        }
        let held = self
            .script
            .as_ref()
            .map_or(self.frame_input.speedbreaker, |run| run.script.input_at(run.time).speedbreaker);
        let speed = self.sim.as_ref().map_or(0.0, |sim| sim.speedbreaker_speed());
        self.breaker.step(held, speed, 0.0, 0.0);
        self.breaker.active()
    }

    /// Recharge uses simulation time; depletion uses the unscaled real frame delta.
    pub fn finish_time(&mut self, input: &ActionState, sim_dt: f32, real_dt: f32) {
        if self.request.is_some() || self.sim.is_none() || (self.batch_run && self.script_finished()) {
            return;
        }
        let held = self
            .script
            .as_ref()
            .map_or(self.frame_input.speedbreaker, |run| run.script.input_at(run.time).speedbreaker);
        let speed = self.sim.as_ref().map_or(0.0, |sim| sim.speedbreaker_speed());
        let active = self.breaker.step(
            held && !self.arrested && !self.breaker_barred && self.cinematic_input.is_none(),
            if self.arrested { 0.0 } else { speed },
            sim_dt,
            real_dt,
        );
        self.breaker_camera.update(active, real_dt);
        if let Some(sim) = self.sim.as_mut() {
            sim.set_nos_recharge(!active);
            sim.set_speedbreaker_effect(active, &self.breaker.tuning().physics);
        }
        let scale = match real_dt > 0.0 {
            true => sim_dt / real_dt,
            false => 0.0,
        };
        self.look.note((input.value(Action::LookX) * scale, input.value(Action::LookY) * scale));
    }
}

#[cfg(test)]
mod tests {
    use super::super::FixedClock;
    use super::super::speedbreaker::SpeedBreaker;
    use super::*;

    #[test]
    fn frame_clock_integrates_entry_and_exit_with_original_ramp_constants() {
        let mut clock = SpeedbreakerClock::default();
        assert!((clock.advance(true, 0.375) - 0.234375).abs() < 1e-6);
        assert!((clock.advance(true, 1.0) - 0.25).abs() < 1e-6);
        assert!((clock.advance(false, 1.5) - 0.9375).abs() < 1e-6);
        assert!((clock.advance(false, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn frame_rates_share_simulation_time_and_real_meter_depletion() {
        for fps in [5, 30, 60, 120, 144, 240] {
            let mut clock = SpeedbreakerClock::default();
            let mut fixed = FixedClock::default();
            let mut meter = SpeedBreaker::default();
            let mut ticks = 0;
            let mut elapsed = 0.0;
            for index in 0..fps * 2 {
                let dt = 1.0 / fps as f32;
                let active = meter.step(index == 0, 50.0, 0.0, dt);
                let sim_dt = clock.advance(active, dt);
                elapsed += sim_dt;
                ticks += fixed.advance(sim_dt);
            }
            assert!((elapsed - 0.640625).abs() < 1e-5, "{fps} FPS: {elapsed}");
            assert_eq!(ticks, 38, "{fps} FPS");
            assert!((meter.meter - 0.8).abs() < 1e-4, "{fps} FPS: {}", meter.meter);
        }
    }

    #[test]
    fn zero_tick_frames_consume_meter_and_pauses_preserve_clock_state() {
        let mut clock = SpeedbreakerClock::default();
        let mut fixed = FixedClock::default();
        let mut meter = SpeedBreaker::default();
        let active = meter.step(true, 50.0, 0.0, 1.0 / 240.0);
        assert_eq!(fixed.advance(clock.advance(active, 1.0 / 240.0)), 0);
        assert!(meter.meter < 1.0 && active);
        let before = clock.multiplier;
        assert_eq!(clock.advance(false, 0.0), 0.0);
        assert_eq!(clock.advance(false, f32::NAN), 0.0);
        assert_eq!(clock.multiplier, before);
        assert!((SpeedbreakerClock::default().advance(false, 2.0) - 2.0).abs() < 1e-6);
    }
}
