//! The chase tuning read from the attribute database: the thresholds of `speechtune` once, and the roadblock and 911
//! fields of each heat level when the chase first reaches it.

use std::collections::HashMap;

use blackbox_attrib::Database;
use nfsmw_data::speech_chase::{ChaseTune, LevelSpeech, chase_tune, level_speech};

#[derive(Default)]
pub struct ChaseData {
    tune: Option<ChaseTune>,
    levels: HashMap<u32, LevelSpeech>,
}

impl ChaseData {
    /// The tuning, once: `Some` the first time it is read from `db`.
    pub fn take_new_tune(&mut self, db: &Database) -> Option<ChaseTune> {
        if self.tune.is_some() {
            return None;
        }
        let tune = chase_tune(db);
        self.tune = Some(tune);
        Some(tune)
    }

    /// The speech fields of heat level `heat`, read from `db` the first time.
    pub fn level(&mut self, db: &Database, heat: u32) -> LevelSpeech {
        *self.levels.entry(heat.clamp(1, 10)).or_insert_with(|| level_speech(db, heat))
    }
}
