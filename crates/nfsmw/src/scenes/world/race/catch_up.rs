//! Quick-race and career catch-up tables and sampled history (`docs/specs/ai-racers-catchup.md`).

use blackbox_driver::{PidError, Table};

const EASY_SPREAD: Table<5> = Table { data: [1000.0, 900.0, 750.0, 600.0, 800.0], min: 0.0, max: 100.0 };
const HARD_SPREAD: Table<5> = Table { data: [300.0, 275.0, 250.0, 150.0, 250.0], min: 0.0, max: 100.0 };
const EASY_SKILL: Table<5> = Table { data: [0.75, 0.75, 0.75, 0.5, 0.25], min: 0.0, max: 100.0 };
const HARD_SKILL: Table<5> = Table { data: [0.75, 0.75, 0.75, 0.75, 0.25], min: 0.0, max: 100.0 };

/// The race context that picks the Easy-to-Hard factor (section 4 of the spec).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scenes::world) enum Context {
    /// A quick race at difficulty class 0, 1 or 2.
    Quick { difficulty: usize },
    /// A career race. `adaptive` is the profile's adaptive bonus (-1..1); `boss` is the race's `BossRace` flag.
    Career { adaptive: f32, boss: bool },
}

impl Context {
    /// The interpolation factor between the Easy (0) and Hard (1) tables.
    pub fn factor(self) -> f32 {
        match self {
            Context::Quick { difficulty } => difficulty.min(2) as f32 * 0.5,
            Context::Career { adaptive, boss } => {
                let adaptive = if adaptive.is_finite() { adaptive } else { 0.0 };
                let factor = ((adaptive + 1.0) * 0.5).clamp(0.0, 1.0);
                if !boss {
                    return factor;
                }
                factor + (1.0 - factor) * 0.5
            }
        }
    }
}

pub(in crate::scenes::world) struct Glue {
    clock: f32,
    history: PidError,
    difficulty: f32,
}

impl Glue {
    pub fn new(context: Context) -> Self {
        Self { clock: 0.0, history: PidError::new(10, 5, 1.0), difficulty: context.factor() }
    }

    pub fn with_phase(mut self, phase: f32) -> Self {
        if phase.is_finite() {
            self.clock = phase.clamp(0.0, 1.0 - f32::EPSILON);
        }
        self
    }

    /// Records once per second; evaluates progress-dependent limits between samples too.
    pub fn update(&mut self, error: f32, dt: f32, percent: f32, emulated: bool, enabled: bool) -> f32 {
        self.clock += dt;
        let interval = (self.clock > 1.0).then_some(self.clock);
        if interval.is_some() {
            self.clock -= 1.0;
        }
        if let Some(interval) = interval {
            self.history.record(error, interval);
        }
        let difficulty = match emulated {
            true => 1.0,
            false => self.difficulty,
        };
        let h = &self.history;
        let output = quick_output(h.error(), h.integral(), h.derivative(), percent, difficulty).1;
        if !enabled && !emulated {
            return 0.0;
        }
        output
    }
}

fn parameters(percent: f32, difficulty: f32) -> (f32, f32) {
    let mix = |easy: &Table<5>, hard: &Table<5>| {
        let low = easy.lookup(percent);
        low + (hard.lookup(percent) - low) * difficulty
    };
    (mix(&EASY_SPREAD, &HARD_SPREAD), mix(&EASY_SKILL, &HARD_SKILL))
}

/// Normalized output and raw skill offset, before the driver's base-skill scaling.
fn quick_output(error: f32, integral: f32, derivative: f32, percent: f32, difficulty: f32) -> (f32, f32) {
    let (spread, amplitude) = parameters(percent, difficulty);
    let output = (2.0 * error / spread.max(100.0) + 0.00005 * integral + 0.01 * derivative).clamp(-1.0, 1.0);
    (output, output * amplitude)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick(difficulty: usize) -> Glue {
        Glue::new(Context::Quick { difficulty })
    }

    #[test]
    fn difficulty_and_progress_select_native_spread_and_skill_limits() {
        assert_eq!(parameters(0.0, 0.0), (1000.0, 0.75));
        assert_eq!(parameters(0.0, 0.5), (650.0, 0.75));
        assert_eq!(parameters(0.0, 1.0), (300.0, 0.75));
        assert_eq!(parameters(62.5, 1.0), (200.0, 0.75));
        assert_eq!(parameters(75.0, 0.0), (600.0, 0.5));
        assert_eq!(parameters(100.0, 1.0), (250.0, 0.25));
    }

    #[test]
    fn an_early_leader_slows_by_at_most_three_quarters_before_driver_scaling() {
        assert_eq!(quick(2).update(-10_000.0, 1.01, 20.0, false, true), -0.75);
        assert_eq!(quick(2).update(10_000.0, 1.01, 20.0, false, true), 0.75);
    }

    #[test]
    fn the_first_sample_uses_trapezoid_area_and_a_smoothed_slope() {
        let output = quick(2).update(10.0, 1.01, 0.0, false, true);
        let expected = (20.0 / 300.0 + 0.00005 * 5.0 + 0.01 * (10.0 / 1.01)) * 0.75;
        assert!((output - expected).abs() < 1e-6);
    }

    #[test]
    fn an_old_gap_expires_from_both_finite_histories() {
        let mut glue = quick(2);
        glue.update(300.0, 1.01, 0.0, false, true);
        for _ in 0..11 {
            glue.update(0.0, 1.0, 0.0, false, true);
        }
        assert_eq!(glue.update(0.0, 0.01, 0.0, false, true), 0.0);
    }

    #[test]
    fn progress_changes_the_output_without_recording_another_error() {
        let mut glue = quick(2);
        assert_eq!(glue.update(10_000.0, 1.01, 0.0, false, true), 0.75);
        assert_eq!(glue.update(0.0, 0.01, 100.0, false, true), 0.25);
    }

    #[test]
    fn an_emulated_easy_rival_uses_the_hard_tables() {
        let simulated = quick(0).update(10.0, 1.01, 75.0, false, true);
        let emulated = quick(0).update(10.0, 1.01, 75.0, true, true);
        let hard = quick(2).update(10.0, 1.01, 75.0, false, true);
        assert_eq!(emulated, hard);
        assert!(emulated > simulated);
    }

    #[test]
    fn career_uses_the_adaptive_factor_and_moves_boss_races_half_way_to_hard() {
        let career = |adaptive, boss| Context::Career { adaptive, boss }.factor();
        assert_eq!(career(0.0, false), 0.5, "a neutral profile plays between Easy and Hard");
        assert_eq!(career(0.0, true), 0.75);
        assert_eq!(career(-1.0, false), 0.0);
        assert_eq!(career(-1.0, true), 0.5);
        assert_eq!(career(1.0, false), 1.0);
        assert_eq!(career(5.0, true), 1.0);
        assert_eq!(career(f32::NAN, false), 0.5);
    }

    #[test]
    fn career_shares_the_quick_race_history_and_tables() {
        let mut career = Glue::new(Context::Career { adaptive: 0.0, boss: false });
        let mut medium = quick(1);
        for (error, dt, percent) in [(400.0, 1.01, 10.0), (-200.0, 1.0, 40.0), (0.0, 0.3, 80.0), (50.0, 1.0, 95.0)] {
            assert_eq!(career.update(error, dt, percent, false, true), medium.update(error, dt, percent, false, true));
        }
        let rival = Glue::new(Context::Career { adaptive: 0.0, boss: true }).update(1000.0, 1.01, 75.0, false, true);
        let hard = quick(2).update(1000.0, 1.01, 75.0, false, true);
        let mid = quick(1).update(1000.0, 1.01, 75.0, false, true);
        assert!(rival > mid && rival < hard, "{mid} < {rival} < {hard}");
    }

    #[test]
    fn off_retains_history_for_forced_emulation_and_clears_it_from_physical_output() {
        let mut glue = quick(2);
        assert_eq!(glue.update(1000.0, 1.01, 0.0, false, false), 0.0);
        assert_eq!(glue.update(1000.0, 0.01, 0.0, true, false), 0.75);
        assert_eq!(glue.update(1000.0, 0.01, 0.0, false, false), 0.0);
    }

    #[test]
    fn independent_initial_phases_change_first_sample_without_advancing_time() {
        assert_eq!(quick(2).update(1000.0, 0.3, 0.0, false, true), 0.0);
        assert_eq!(quick(2).with_phase(0.75).update(1000.0, 0.3, 0.0, false, true), 0.75);
    }

    #[test]
    #[ignore = "requires external asset-free NFSMW_RACER_CATCHUP_TRACE from a Hard PC capture"]
    fn recorded_hard_quick_catchup_outputs_match() {
        type Row = (u32, f32, f32, f32, f32, f32, bool, f32, f32);
        let path = std::env::var_os("NFSMW_RACER_CATCHUP_TRACE").unwrap();
        let rows: Vec<Row> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(rows.len() > 1000);
        for (id, time, percent, error, integral, derivative, _forced, normalized, skill) in rows {
            let actual = quick_output(error, integral, derivative, percent, 1.0);
            assert!(
                (actual.0 - normalized).abs() < 2e-5,
                "racer {id} at {time}: normalized {actual:?} != {normalized}"
            );
            assert!((actual.1 - skill).abs() < 2e-5, "racer {id} at {time}: skill {actual:?} != {skill}");
        }
    }
}
