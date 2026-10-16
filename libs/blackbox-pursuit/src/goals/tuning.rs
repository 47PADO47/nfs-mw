//! The tuning a goal reads from the heat row (`pursuitlevels`) and the cop's data. The defaults are the spec's values for a
//! level that has no row. Spec: `docs/specs/ai-pursuit-goals.md` sections 2.1, 2.2, 7.1 and 8.

use super::pit;
use super::record::GoalKind;

/// The goal tuning of one pursuit level. Each heat row fills these; a missing row takes the defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// `BoxinTightness` `t`, 0 loose to 1 tight. The box-in scales its wings by `0.7 - 0.5 t` and its depth by `2 - 5 t`.
    pub boxin_tightness: f32,
    /// `BoxinDuration` in seconds, the box-in finisher's run. The code stores it; the rules here read it through
    /// [`Tuning::finisher_duration`].
    pub boxin_duration: f32,
    /// `RollingBlockTightness` `t`. The rolling block scales its lateral in-position offsets by `1 - 0.8 t`.
    pub rolling_block_tightness: f32,
    /// `RollingBlockDuration` in seconds, the rolling block's finisher run.
    pub rolling_block_duration: f32,
    /// `CollapseAggression`, 0 to 1, of the target's pursuit level. The code reads 0 when the level has no row.
    pub collapse_aggression: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            boxin_tightness: 0.5,
            boxin_duration: 2.0,
            rolling_block_tightness: 0.5,
            rolling_block_duration: 2.0,
            collapse_aggression: 0.0,
        }
    }
}

impl Tuning {
    /// The finisher's run time in seconds for a goal kind. The pit runs a fixed 2.0 s; the pull-over has no finisher.
    pub fn finisher_duration(&self, kind: GoalKind) -> Option<f32> {
        match kind {
            GoalKind::Pit => Some(pit::RUN_TIME),
            GoalKind::BoxIn => Some(self.boxin_duration),
            GoalKind::RollingBlock => Some(self.rolling_block_duration),
            GoalKind::PullOver => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_spec_values() {
        let t = Tuning::default();
        assert_eq!(t.boxin_tightness, 0.5);
        assert_eq!(t.boxin_duration, 2.0);
        assert_eq!(t.rolling_block_tightness, 0.5);
        assert_eq!(t.rolling_block_duration, 2.0);
        assert_eq!(t.collapse_aggression, 0.0, "the code reads 0 when the row is missing");
    }

    #[test]
    fn the_finisher_run_depends_on_the_goal_kind() {
        let t = Tuning { boxin_duration: 6.0, rolling_block_duration: 9.0, ..Tuning::default() };
        assert_eq!(t.finisher_duration(GoalKind::BoxIn), Some(6.0));
        assert_eq!(t.finisher_duration(GoalKind::RollingBlock), Some(9.0));
        assert_eq!(t.finisher_duration(GoalKind::Pit), Some(2.0));
        assert_eq!(t.finisher_duration(GoalKind::PullOver), None, "the collapse has no finisher");
    }
}
