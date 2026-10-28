//! The overrun backfire of a mod car (`[car_sound.overrun]`, docs/modding/car-sound.md, "The overrun backfire"): a charge
//! that builds while the engine is fed and drains while it is not decides the bangs and flames of a shut throttle
//! (`blackbox_carsound::overrun`, spec `docs/specs/ride6-engine-sound.md` section 5). This module turns the manifest's
//! numbers into the controller's tuning, feeds it the engine speed and the throttle, and says what the sound side does with
//! what comes out; it plays nothing itself, so it is tested without an audio device.
//!
//! What the host does with a result, in one place:
//!
//! - **Bang:** one random file of `backfire` plays at [`bang_volume`] of the bang's audio level; a bang with an effect
//!   level also lights the tail pipes once, and the one to three pops that follow (the core meaning of `pops`) light them
//!   too; a bang without an effect level, and its pops, are audio only.
//! - **Flame:** plays nothing (the bank the table was made from has no flame recording for the cars that use it); a flame
//!   with an effect level lights the pipes, once every [`FLAME_PULSE_SECONDS`] while it burns.

use blackbox_carsound::overrun::{OverrunBackfire, OverrunInput, OverrunTuning};
use nfsmw_data::car::mods::Overrun;

use super::logic::{self, Pop, Rng};

/// A flame that burns lights the pipes this often. The game's pipe flame is a short burst per bang (0.06 s,
/// docs/exhaust-flames.md), so a continuous flame is a burst each time the last one ends. A modelling choice: the pipes
/// take bangs, not a strength, so the flame's duration is rebuilt from bangs.
pub const FLAME_PULSE_SECONDS: f32 = 0.06;

/// The tuning the manifest's numbers make.
pub fn tuning_of(table: &Overrun) -> OverrunTuning {
    OverrunTuning {
        accumulator_rate: table.accumulator_rate,
        throttle_arm: table.throttle_arm,
        throttle_fire: table.throttle_fire,
        window_seconds: table.window_seconds,
        flame_delay: table.flame_delay,
        flame_min_rpm: table.flame_min_rpm,
        bang_audio_threshold: table.bang_audio_threshold,
        bang_effect_threshold: table.bang_effect_threshold,
        flame_effect_threshold: table.flame_effect_threshold,
        probability_closed: table.probability_closed,
        probability_open: table.probability_open,
    }
}

/// The volume a recording plays at for a bang of audio level `level` (0 to 1): the level itself. A modelling choice
/// (spec section 7): the original hands the level to its sound engine as a parameter whose effect on the recordings is
/// authored in sound banks that are not readable, so the level is read as a gain.
pub fn bang_volume(level: f32) -> f32 {
    if !level.is_finite() {
        return 0.0;
    }
    level.clamp(0.0, 1.0)
}

/// A bang to play.
#[derive(Debug, Clone, PartialEq)]
pub struct BangPlay {
    /// The volume of the recording (0 to 1, before the host's one-shot level).
    pub volume: f32,
    /// The bang, and the pops after it, light the tail pipes.
    pub pipes: bool,
    /// The pops that follow, in time order.
    pub pops: Vec<Pop>,
}

/// What one frame asks the sound side to do.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OverrunPlay {
    pub bang: Option<BangPlay>,
    /// A flame lights the tail pipes now.
    pub flame_pulse: bool,
}

/// The overrun controller of a car that is playing.
pub struct OverrunDrive {
    fx: OverrunBackfire,
    idle_rpm: f32,
    redline_rpm: f32,
    /// Seconds until a burning flame lights the pipes again.
    pulse_wait: f32,
}

impl OverrunDrive {
    /// `idle_rpm` and `redline_rpm` are what the host's `0 to 1` engine speed spans, so that the engine speed over the
    /// redline (the controller's `rpm_fraction`) can be told from it. `seed` makes the bang draws repeat.
    pub fn new(table: &Overrun, idle_rpm: f32, redline_rpm: f32, seed: u32) -> Self {
        Self { fx: OverrunBackfire::new(tuning_of(table), seed), idle_rpm, redline_rpm, pulse_wait: 0.0 }
    }

    /// The engine speed over its redline for `rpm_pct`, the host's `(rpm - idle) / (redline - idle)`.
    pub fn rpm_fraction(&self, rpm_pct: f32) -> f32 {
        if !rpm_pct.is_finite() || self.redline_rpm <= 0.0 {
            return 0.0;
        }
        let rpm = self.idle_rpm + rpm_pct.clamp(0.0, 1.0) * (self.redline_rpm - self.idle_rpm);
        (rpm / self.redline_rpm).clamp(0.0, 1.0)
    }

    /// One frame of `dt` seconds. `throttle` is the throttle pedal; the host has no separate electronic throttle, so it
    /// stands for both the engine's and the driver's. The pops after a bang draw from `rng`.
    pub fn step(&mut self, dt: f32, throttle: f32, rpm_pct: f32, rng: &mut Rng) -> OverrunPlay {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let out = self.fx.update(OverrunInput {
            dt,
            throttle,
            raw_throttle: throttle,
            rpm_fraction: self.rpm_fraction(rpm_pct),
        });
        self.pulse_wait = (self.pulse_wait - dt).max(0.0);
        let lit = out.flame.is_some_and(|flame| flame.effect_level.is_some());
        let flame_pulse = lit && self.pulse_wait <= 0.0;
        if flame_pulse {
            self.pulse_wait = FLAME_PULSE_SECONDS;
        }
        let bang = out.bang.map(|bang| {
            let pipes = bang.effect_level.is_some();
            BangPlay { volume: bang_volume(bang.audio_level), pipes, pops: logic::pops_after_bang(rng) }
        });
        OverrunPlay { bang, flame_pulse }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers of the Custom bike the table was made for, with the chance at 1 so that a draw cannot hide a bang.
    fn table() -> Overrun {
        Overrun {
            accumulator_rate: 0.2,
            throttle_arm: 0.9,
            throttle_fire: 0.1,
            window_seconds: 2.0,
            flame_delay: 0.2,
            flame_min_rpm: 0.1,
            bang_audio_threshold: 0.95,
            bang_effect_threshold: 0.8,
            flame_effect_threshold: 1.0,
            probability_closed: 1.0,
            probability_open: 1.0,
        }
    }

    fn drive(table: &Overrun) -> OverrunDrive {
        OverrunDrive::new(table, 2300.0, 11000.0, 5)
    }

    /// Seconds at full throttle and the redline, in 60 Hz frames.
    fn pull(fx: &mut OverrunDrive, rng: &mut Rng, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            fx.step(1.0 / 60.0, 1.0, 1.0, rng);
        }
    }

    #[test]
    fn the_engine_speed_over_the_redline_is_told_from_the_host_s_fraction() {
        let fx = drive(&table());
        assert_eq!(fx.rpm_fraction(1.0), 1.0);
        assert!((fx.rpm_fraction(0.0) - 2300.0 / 11000.0).abs() < 1e-6);
        assert!((fx.rpm_fraction(0.5) - 6650.0 / 11000.0).abs() < 1e-6);
        assert_eq!((fx.rpm_fraction(f32::NAN), fx.rpm_fraction(9.0)), (0.0, 1.0));
    }

    #[test]
    fn a_hard_pull_and_an_instant_lift_bang_with_pops_and_flash_the_pipes() {
        let (mut fx, mut rng) = (drive(&table()), Rng::new(9));
        pull(&mut fx, &mut rng, 6.0);
        let play = fx.step(1.0 / 60.0, 0.0, 1.0, &mut rng);
        let bang = play.bang.expect("a bang");
        assert!(bang.volume > 0.9 && bang.pipes, "{bang:?}");
        assert!((1..=3).contains(&bang.pops.len()));
        assert!(bang.pops.windows(2).all(|pair| pair[1].delay > pair[0].delay));
        assert!(bang.pops[0].delay >= 0.12);
        assert!(!play.flame_pulse, "the stock flame threshold of 1 never lights the pipes");
    }

    #[test]
    fn a_short_pull_or_a_slow_lift_stays_quiet() {
        let (mut fx, mut rng) = (drive(&table()), Rng::new(9));
        pull(&mut fx, &mut rng, 2.0);
        assert!(fx.step(1.0 / 60.0, 0.0, 1.0, &mut rng).bang.is_none(), "a two second pull charges to 0.4");
        let (mut fx, mut rng) = (drive(&table()), Rng::new(9));
        pull(&mut fx, &mut rng, 6.0);
        let mut bang = None;
        for _ in 0..120 {
            bang = bang.or(fx.step(1.0 / 60.0, 0.5, 1.0, &mut rng).bang);
        }
        bang = bang.or(fx.step(1.0 / 60.0, 0.0, 1.0, &mut rng).bang);
        assert!(bang.is_none(), "two seconds of lifting is outside the window");
    }

    #[test]
    fn a_bang_below_the_effect_threshold_is_audio_only() {
        let mut quiet = table();
        quiet.bang_audio_threshold = 0.3;
        quiet.bang_effect_threshold = 0.99;
        let (mut fx, mut rng) = (drive(&quiet), Rng::new(9));
        pull(&mut fx, &mut rng, 3.0);
        let bang = fx.step(1.0 / 60.0, 0.0, 1.0, &mut rng).bang.expect("a bang");
        assert!(!bang.pipes && bang.volume > 0.0);
    }

    #[test]
    fn a_burning_flame_with_an_effect_threshold_pulses_the_pipes_at_the_pulse_rate() {
        let mut lit = table();
        lit.flame_effect_threshold = 0.0;
        let (mut fx, mut rng) = (drive(&lit), Rng::new(9));
        pull(&mut fx, &mut rng, 6.0);
        let mut pulses = 0;
        for _ in 0..120 {
            pulses += usize::from(fx.step(1.0 / 60.0, 0.0, 1.0, &mut rng).flame_pulse);
        }
        // Two seconds of a shut throttle, the flame starting after 0.2 s, a pulse every fourth 60 Hz frame (the first frame
        // past 0.06 s): about 27, and never more than one per pulse time.
        assert!((20..=31).contains(&pulses), "{pulses}");
        assert!(!fx.step(1.0 / 60.0, 0.8, 1.0, &mut rng).flame_pulse, "the closing frame is a zero flame");
    }

    #[test]
    fn the_same_seed_gives_the_same_bangs() {
        let run = |seed| {
            let mut half = table();
            half.probability_closed = 0.5;
            half.probability_open = 0.5;
            let (mut fx, mut rng) = (OverrunDrive::new(&half, 2300.0, 11000.0, seed), Rng::new(seed));
            let mut seen = Vec::new();
            for _ in 0..30 {
                pull(&mut fx, &mut rng, 6.0);
                seen.push(fx.step(1.0 / 60.0, 0.0, 1.0, &mut rng).bang);
                fx.step(1.0 / 60.0, 0.0, 0.0, &mut rng);
            }
            seen
        };
        assert_eq!(run(3), run(3));
        assert!(run(3).iter().any(Option::is_some) && run(3).iter().any(Option::is_none));
    }

    #[test]
    fn the_volume_is_the_level_held_to_range() {
        assert_eq!(
            (bang_volume(0.4), bang_volume(3.0), bang_volume(-1.0), bang_volume(f32::NAN)),
            (0.4, 1.0, 0.0, 0.0)
        );
    }
}
