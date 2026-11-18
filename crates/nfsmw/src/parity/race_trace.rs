//! The race trace: one JSON line per thing a race did, for comparing a race run in the remake with the same race run
//! in the original game. Off unless `--race-trace <file>` or `NFSMW_RACE_TRACE=<file>` names a file.
//!
//! The first line of a race is its `header`: the run header every trace shares (`header.rs`), the event, the racers and
//! the laps. Then `timer_start`, `autopilot`, `checkpoint`, `lap`, `racer_finished`, `knocked_out` and `race_finished`
//! lines follow, each stamped with the physics tick it happened in, the simulated time and the wall time. Racer 0 is the
//! player; laps and checkpoints count from 1; times are seconds of race time, which starts at the green light.
//!
//! A race raises a few lines a second at most, so they are written and flushed where they are raised: a line is on
//! disk when the process ends, with no thread to wait for.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use blackbox_race::{Race, RaceEvent};
use serde::Serialize;

use super::header::RunHeader;
use super::race_run::Autopilot;

/// The layout's name, in every header.
pub const SCHEMA: &str = "nfsmw-race-trace-v1";

/// When a line happened.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Stamp {
    /// The physics tick the line was raised in, counted from 1 since the world opened. Ticks do not run while the
    /// world loads or is paused.
    pub tick: u64,
    /// Simulated seconds since the world opened: the sum of the ticks' simulated deltas.
    pub sim_time: f64,
    /// Wall time: monotonic milliseconds since the trace file was opened.
    pub wall_ms: f64,
}

/// One racer on the grid.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RacerRow {
    pub racer: usize,
    pub name: String,
    /// The car: the player's car folder, an opponent's `pvehicle` name.
    pub car: Option<String>,
    /// The race counts this racer as a person: the race is over when every such racer is done.
    pub human: bool,
}

/// How one racer stood when the race was over.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ResultRow {
    pub racer: usize,
    pub place: u32,
    pub finished: bool,
    /// The finish time; `null` for a racer that did not finish before the race was over.
    pub total_time: Option<f32>,
}

/// What a race's first line says about it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Header {
    pub schema: &'static str,
    /// Unix time of the line, milliseconds.
    pub created_unix_ms: u64,
    pub run: RunHeader,
    /// The career event id, for example `16.1.1`.
    pub event: String,
    /// The race's collection name.
    pub race: String,
    pub race_type: String,
    /// The opponents that stand on the grid.
    pub opponents: usize,
    pub laps: u32,
    /// The checkpoints of one lap, without the finish line.
    pub checkpoints_per_lap: usize,
    /// The race has a finish line besides its checkpoints; crossing it closes a lap and raises no `checkpoint` line.
    pub finish_line: bool,
    /// No countdown: the timer starts when the racers are placed.
    pub rolling_start: bool,
    pub racers: Vec<RacerRow>,
    /// The autopilot of the run; `null` when a person (or nobody) drives the player's car.
    pub autopilot: Option<AutopilotRow>,
}

/// The autopilot's options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AutopilotRow {
    pub after_secs: f32,
    pub skill: f32,
}

impl From<Autopilot> for AutopilotRow {
    fn from(autopilot: Autopilot) -> Self {
        Self { after_secs: autopilot.after_secs, skill: autopilot.skill }
    }
}

/// One line of the trace.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Record {
    Header {
        #[serde(flatten)]
        stamp: Stamp,
        #[serde(flatten)]
        header: Box<Header>,
    },
    /// The green light: the race timer starts and the racers are let go.
    TimerStart {
        #[serde(flatten)]
        stamp: Stamp,
        race_time: f32,
    },
    /// The autopilot took the player's car.
    Autopilot {
        #[serde(flatten)]
        stamp: Stamp,
        race_time: f32,
    },
    Checkpoint {
        #[serde(flatten)]
        stamp: Stamp,
        racer: usize,
        lap: u32,
        checkpoint: usize,
        race_time: f32,
    },
    /// A lap was completed, the last one included.
    Lap {
        #[serde(flatten)]
        stamp: Stamp,
        racer: usize,
        lap: u32,
        lap_time: f32,
        race_time: f32,
    },
    RacerFinished {
        #[serde(flatten)]
        stamp: Stamp,
        racer: usize,
        place: u32,
        total_time: f32,
    },
    KnockedOut {
        #[serde(flatten)]
        stamp: Stamp,
        racer: usize,
        race_time: f32,
    },
    /// The race is over: every person is done, or the time ran out. Racers still on the road are not timed after it.
    RaceFinished {
        #[serde(flatten)]
        stamp: Stamp,
        race_time: f32,
        results: Vec<ResultRow>,
    },
}

/// Turns a race's events into lines. It counts each racer's laps and checkpoints itself, in the order the events were
/// raised, so a checkpoint and a lap raised in the same tick are numbered as they happened.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaceLog {
    /// Per racer: the laps completed and the checkpoints reached so far.
    counts: Vec<(u32, usize)>,
}

impl RaceLog {
    /// The lines of `event`, which `race` has just raised. Events the trace does not record give none.
    pub fn records(&mut self, race: &Race, event: &RaceEvent, stamp: Stamp) -> Vec<Record> {
        let now = race.elapsed();
        match *event {
            RaceEvent::Go => vec![Record::TimerStart { stamp, race_time: now }],
            RaceEvent::Checkpoint { racer, number } => {
                let (laps, reached) = self.count(racer);
                let race_time = race.racers().get(racer).and_then(|r| r.checkpoint_times.get(reached)).copied();
                self.counts[racer].1 += 1;
                vec![Record::Checkpoint {
                    stamp,
                    racer,
                    lap: laps + 1,
                    checkpoint: number,
                    race_time: race_time.unwrap_or(now),
                }]
            }
            RaceEvent::Lap { racer, .. } => vec![self.lap(race, racer, stamp, now)],
            RaceEvent::Finished { racer, time } => {
                let place = race.places().get(racer).copied().unwrap_or(0);
                vec![
                    self.lap(race, racer, stamp, time),
                    Record::RacerFinished { stamp, racer, place, total_time: time },
                ]
            }
            RaceEvent::KnockedOut { racer } => vec![Record::KnockedOut { stamp, racer, race_time: now }],
            RaceEvent::Over => {
                let results = race.outcomes().into_iter().map(|outcome| ResultRow {
                    racer: outcome.racer,
                    place: outcome.place,
                    finished: outcome.finished,
                    total_time: outcome.time,
                });
                vec![Record::RaceFinished { stamp, race_time: now, results: results.collect() }]
            }
            RaceEvent::Count(_)
            | RaceEvent::TimeUp
            | RaceEvent::Bonus { .. }
            | RaceEvent::SpeedTrap { .. }
            | RaceEvent::Reset { .. } => Vec::new(),
        }
    }

    /// The laps and checkpoints counted for `racer` so far.
    fn count(&mut self, racer: usize) -> (u32, usize) {
        if self.counts.len() <= racer {
            self.counts.resize(racer + 1, (0, 0));
        }
        self.counts[racer]
    }

    /// The line of the lap `racer` has just closed, at the race time the race kept for it (`fallback` without one).
    fn lap(&mut self, race: &Race, racer: usize, stamp: Stamp, fallback: f32) -> Record {
        let (laps, _) = self.count(racer);
        let times = race.racers().get(racer).map_or(&[][..], |r| r.lap_times.as_slice());
        let end = times.get(laps as usize).copied().unwrap_or(fallback);
        let begin = laps.checked_sub(1).and_then(|last| times.get(last as usize)).copied().unwrap_or(0.0);
        self.counts[racer].0 += 1;
        Record::Lap { stamp, racer, lap: laps + 1, lap_time: end - begin, race_time: end }
    }
}

/// The trace file. A line that cannot be written is reported once and ends the trace; the game goes on.
pub struct TraceFile {
    path: PathBuf,
    out: Option<File>,
    opened: Instant,
}

impl TraceFile {
    /// Creates `path` (and its folder), replacing a file that is there.
    pub fn create(path: &Path) -> std::io::Result<Self> {
        if let Some(folder) = path.parent().filter(|folder| !folder.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder)?;
        }
        Ok(Self { path: path.to_owned(), out: Some(File::create(path)?), opened: Instant::now() })
    }

    /// Milliseconds since the file was opened, for a line's stamp.
    pub fn wall_ms(&self) -> f64 {
        self.opened.elapsed().as_secs_f64() * 1000.0
    }

    /// Appends `record` as one line and flushes it.
    pub fn write(&mut self, record: &Record) {
        let Some(out) = self.out.as_mut() else { return };
        let written = serde_json::to_vec(record).map_err(std::io::Error::other).and_then(|mut line| {
            line.push(b'\n');
            out.write_all(&line).and_then(|()| out.flush())
        });
        if let Err(e) = written {
            log::error!("race trace: writing {} failed, the trace ends here: {e}", self.path.display());
            self.out = None;
        }
    }
}

#[cfg(test)]
mod tests;
