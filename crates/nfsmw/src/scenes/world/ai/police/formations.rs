//! Selection gates and heat-aware finisher state. Spec: `ai-pursuit-formations.md` sections 2, 5 and 8.

use blackbox_pursuit::formation::finisher::{Event, Finisher, Params, params_for};
use blackbox_pursuit::formation::{Formation, Record, collapse};
use blackbox_pursuit::goals::Tuning;

use super::Police;
use crate::scenes::world::ai::goal_aims;

/// The radio reports setup focus while it is active; formation changes wait fifteen seconds afterwards.
pub(super) struct SpeechGate {
    enabled: bool,
    active: bool,
    since_setup: f32,
}

impl Default for SpeechGate {
    fn default() -> Self {
        Self { enabled: false, active: false, since_setup: 15.0 }
    }
}

impl Police {
    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn test_formation(&mut self, formation: Formation, running: bool) {
        self.last_formation = Some(formation);
        self.finisher = Finisher::default();
        if running && let Some(params) = self.finisher_params() {
            self.finisher.tick(params.time_to_arm, params.min_cops, Some(0.0), &params);
        }
    }

    pub(in crate::scenes::world::ai) fn hash_formations(&self, hash: &mut crate::scenes::world::StateHasher) {
        hash.f32(self.formations.remaining_time());
        hash.u32(self.formations.attempts());
        hash.bytes(format!("{:?}", self.last_formation).as_bytes());
        hash.u32(self.last_level);
        hash.f32(self.finisher.in_position());
        hash.bool(self.finisher.running_time().is_some());
        if let Some(time) = self.finisher.running_time() {
            hash.f32(time);
        }
        hash.bool(self.formation_speech.active);
        hash.bool(self.formation_speech.enabled);
        hash.f32(self.formation_speech.since_setup);
    }

    /// Install the ten aligned heat rows of the selected free-roam or race escalation table.
    pub fn set_formation_rows(&mut self, rows: Vec<nfsmw_data::pursuit::FormationRow>) {
        self.formation_rows = rows;
        self.formations.heat_changed();
    }

    fn formation_row(&self) -> Option<&nfsmw_data::pursuit::FormationRow> {
        self.formation_rows.get(self.level().unwrap_or(1).saturating_sub(1) as usize)
    }

    /// Shared approach, goal and duration tuning from the active escalation context.
    pub(in crate::scenes::world::ai) fn goal_tuning(&self) -> Tuning {
        let Some(row) = self.formation_row() else { return goal_aims::tuning_for(self.level().unwrap_or(1)) };
        Tuning {
            boxin_tightness: row.boxin_tightness,
            boxin_duration: row.boxin_duration,
            rolling_block_tightness: row.rolling_block_tightness,
            rolling_block_duration: row.rolling_block_duration,
            collapse_aggression: row.collapse_aggression,
        }
    }

    pub(in crate::scenes::world::ai) fn collapse_tuning(&self) -> collapse::Level {
        let Some(row) = self.formation_row() else { return collapse::level(self.level().unwrap_or(1)) };
        collapse::Level {
            speed_kmh: row.collapse_speed,
            inner: row.collapse_inner,
            outer: row.collapse_outer,
            max_cops: row.max_cops_collapsing,
        }
    }

    /// Supply whether the speech controller currently owns setup focus. Disabled speech supplies false.
    pub fn set_speech_enabled(&mut self, enabled: bool) {
        self.formation_speech.enabled = enabled;
        if !enabled {
            self.formation_speech.active = false;
            self.formation_speech.since_setup = 15.0;
        }
    }

    /// Supply whether enabled speech currently owns setup focus.
    pub fn set_speech_setup(&mut self, active: bool) {
        self.formation_speech.active = active;
        if active {
            self.formation_speech.since_setup = 0.0;
        }
    }

    fn finisher_params(&self) -> Option<Params> {
        let formation = self.last_formation?;
        let mut params = params_for(formation)?;
        let tuning = self.goal_tuning();
        params.run_time = match formation {
            Formation::BoxIn => tuning.boxin_duration,
            Formation::RollingBlock => tuning.rolling_block_duration,
            _ => params.run_time,
        };
        Some(params)
    }

    /// Remaining approach fraction: one when arming starts and zero when the slots have been held long enough.
    pub(in crate::scenes::world::ai) fn formation_progress(&self) -> f32 {
        let Some(params) = self.finisher_params() else { return 1.0 };
        (1.0 - self.finisher.in_position() / params.time_to_arm).clamp(0.0, 1.0)
    }

    pub(super) fn tick_finisher(&mut self, dt: f32, gap: Option<(u32, f32)>, target_speed: f32) {
        if self.is_busted() || self.end_reason != blackbox_pursuit::EndReason::None {
            self.finisher = Finisher::default();
            return;
        }
        let Some(params) = self.finisher_params() else { return };
        let collapse_speed = match self.is_jerk() {
            true => 125.0,
            false => self.collapse_tuning().speed_kmh,
        };
        // A running finisher has priority; otherwise collapse suppresses the arming timer.
        if !self.finisher.is_running() && target_speed.abs() * 3.6 < collapse_speed {
            self.finisher = Finisher::default();
            return;
        }
        let (count, average) = gap.map_or((0, None), |(count, average)| (count, Some(average)));
        match self.finisher.tick(dt, count, average, &params) {
            Event::Armed => log::info!("the finisher runs: the cops hold their slots"),
            Event::Ended => log::info!("the finisher is over: the cops return to the chase"),
            Event::None => {}
        }
    }

    /// Whether a finisher is running; its assigned cops retain their slot's goal until the run ends.
    pub fn finisher_running(&self) -> bool {
        self.finisher.is_running()
    }

    pub(super) fn tick_formations(&mut self, dt: f32, level: u32, random: [f32; 2]) {
        if !self.formation_speech.active {
            self.formation_speech.since_setup += dt;
        }
        if level != self.last_level {
            self.last_level = level;
            self.formations.heat_changed();
        }
        let speech_done = !self.formation_speech.enabled
            || self.pursuit_context().pursuit_event
            || self.pursuit_context().challenge
            || (!self.formation_speech.active && self.formation_speech.since_setup >= 15.0);
        let allowed = !self.finisher.is_running()
            && !self.is_busted()
            && self.end_reason == blackbox_pursuit::EndReason::None
            && speech_done;
        let choice = match self.formation_row() {
            Some(row) => {
                let stagger = row.stagger_time;
                let records: Vec<Record> = row
                    .records
                    .iter()
                    .filter_map(|record| {
                        let formation = match record.kind {
                            1 => Formation::Pit,
                            2 => Formation::BoxIn,
                            3 => Formation::RollingBlock,
                            4 | 5 => Formation::Follow,
                            6 => Formation::Herd,
                            7 => Formation::StaggerFollow,
                            _ => return None,
                        };
                        Some(Record { formation, duration: record.duration, frequency: record.frequency })
                    })
                    .collect();
                self.formations.tick_with_records(dt, allowed, self.roadblock.follow, random, &records, stagger)
            }
            None => self.formations.tick(dt, level, allowed, self.roadblock.follow, random),
        };
        let Some(choice) = choice else { return };
        if self.last_formation == Some(choice.formation) {
            return;
        }
        log::debug!("cop formation: {:?} for {:.0} s", choice.formation, choice.duration);
        self.last_formation = Some(choice.formation);
        self.finisher = Finisher::default();
    }

    /// The last formation chosen for this chase.
    pub fn last_formation(&self) -> Option<Formation> {
        self.last_formation
    }
}

#[cfg(test)]
mod tests;
