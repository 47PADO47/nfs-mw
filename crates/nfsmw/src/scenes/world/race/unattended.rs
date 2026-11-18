//! An unattended race run in the world (`parity/race_run.rs`): the race the command line asked for is queued as a quick
//! race is, the physics ticks are counted, every race writes its trace (`parity/race_trace.rs`) and the process is asked
//! to end when a race is over. The autopilot is `autopilot.rs`. With no option given, none of this does anything.

use blackbox_race::RaceEvent;

use super::{RaceOptions, RaceSession};
use crate::parity::race_run::{self, RaceRunOptions};
use crate::parity::race_trace::{Header, RaceLog, RacerRow, Record, SCHEMA, Stamp, TraceFile};
use crate::scenes::world::WorldScene;
use crate::scenes::world::ai::MomentDriver;

/// The world's side of an unattended race run.
#[derive(Default)]
pub(in crate::scenes::world) struct Unattended {
    pub(super) plan: RaceRunOptions,
    /// The requested race is queued and has not started yet.
    awaiting_start: bool,
    /// The physics ticks run since the world opened, and their simulated seconds.
    ticks: u64,
    sim_time: f64,
    trace: Option<TraceFile>,
    /// The trace file was asked for once; a file that could not be made is not tried again.
    trace_tried: bool,
    log: RaceLog,
    /// The autopilot's driver, while it has the player's car (`autopilot.rs`).
    pub(super) pilot: Option<MomentDriver>,
    pub(super) engaged: bool,
}

impl Unattended {
    /// The state of this run's world: the run's options, with the race for the first world only.
    pub(in crate::scenes::world) fn of_run() -> Self {
        Self { plan: race_run::take(), ..Self::default() }
    }

    /// One physics tick of `dt` simulated seconds begins.
    pub(in crate::scenes::world) fn note_tick(&mut self, dt: f32) {
        self.ticks += 1;
        self.sim_time += f64::from(dt);
    }

    /// The autopilot has the player's car: a world moment that ends leaves the car with it.
    pub(in crate::scenes::world) fn pilot_engaged(&self) -> bool {
        self.engaged
    }

    fn stamp(&self) -> Stamp {
        let wall_ms = self.trace.as_ref().map_or(0.0, TraceFile::wall_ms);
        Stamp { tick: self.ticks, sim_time: self.sim_time, wall_ms }
    }

    /// The trace file, made when the first race needs it; `None` when the run records none.
    fn trace(&mut self) -> Option<&mut TraceFile> {
        if !std::mem::replace(&mut self.trace_tried, true)
            && let Some(path) = self.plan.trace.as_deref()
        {
            match TraceFile::create(path) {
                Ok(file) => {
                    log::info!("race trace: recording into {}", path.display());
                    self.trace = Some(file);
                }
                Err(e) => log::error!("race trace: cannot create {}: {e}", path.display()),
            }
        }
        self.trace.as_mut()
    }

    /// Writes the line `record` builds from the stamp of now, when the run records a trace.
    pub(super) fn write(&mut self, record: impl FnOnce(Stamp) -> Record) {
        if self.trace().is_none() {
            return;
        }
        let record = record(self.stamp());
        if let Some(trace) = self.trace.as_mut() {
            trace.write(&record);
        }
    }
}

/// The options of the race a run asked for: the event's own but for the opponents and the laps given.
pub(super) fn options_of(request: &race_run::RaceRequest) -> RaceOptions {
    RaceOptions { laps: request.laps, opponents: request.opponents, ..RaceOptions::default() }
}

impl WorldScene {
    /// Queues the race the run asked for, once: it starts as a quick race does, when the player's car exists.
    pub(super) fn queue_run_race(&mut self) {
        let Some(request) = self.unattended.plan.race.take() else { return };
        log::info!("race run: starting {}", request.event);
        let options = options_of(&request);
        self.unattended.awaiting_start = true;
        self.queue_race(request.event, options);
    }

    /// After the queued race was tried: a requested race that did not start ends a run that exits after its race, as a
    /// failure, so a batch job does not wait in free roam for ever.
    pub(super) fn check_run_race_started(&mut self) {
        if !self.unattended.awaiting_start || self.queued_race.is_some() {
            return;
        }
        self.unattended.awaiting_start = false;
        if self.race.is_some() {
            return;
        }
        log::error!("race run: the race asked for with --race did not start");
        if self.unattended.plan.exit_after_race {
            race_run::request_exit(Err("the race asked for with --race did not start (the log says why)".into()));
        }
    }

    /// The racers stand on the grid and the race exists: its header line.
    pub(super) fn trace_race_begins(&mut self, session: &RaceSession) {
        let Some(engine) = session.engine.as_ref() else { return };
        self.unattended.log = RaceLog::default();
        if self.unattended.plan.trace.is_none() {
            return;
        }
        let player_car = self.players.lead().map(|drive| drive.car_name.clone());
        let racers = session.plan.iter().zip(engine.racers()).enumerate().map(|(racer, (plan, state))| RacerRow {
            racer,
            name: plan.name.clone(),
            car: plan.model.clone().or_else(|| player_car.clone().filter(|_| racer == 0)),
            human: state.human,
        });
        let course = engine.course();
        let header = Header {
            schema: SCHEMA,
            created_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_millis() as u64),
            run: self.run_header(),
            event: session.event.clone(),
            race: session.name.clone(),
            race_type: format!("{:?}", session.kind).to_ascii_lowercase(),
            opponents: engine.racers().len().saturating_sub(1),
            laps: course.laps,
            checkpoints_per_lap: course.checkpoints.len(),
            finish_line: course.finish.is_some(),
            rolling_start: session.origin.race.params.rolling_start,
            racers: racers.collect(),
            autopilot: self.unattended.plan.autopilot.map(Into::into),
        };
        self.unattended.write(|stamp| Record::Header { stamp, header: Box::new(header) });
    }

    /// One event of the running race: its trace lines, and the end of the process when the race is over and the run
    /// asked for that.
    pub(super) fn trace_race_event(&mut self, session: &RaceSession, event: &RaceEvent) {
        let Some(engine) = session.engine.as_ref() else { return };
        if self.unattended.plan.trace.is_some() {
            let stamp = self.unattended.stamp();
            for record in self.unattended.log.records(engine, event, stamp) {
                self.unattended.write(|_| record);
            }
        }
        if *event == RaceEvent::Over && self.unattended.plan.exit_after_race {
            log::info!("race run: {} is over, the process ends", session.name);
            race_run::request_exit(Ok(()));
        }
    }

    /// A race was taken down, finished or not: the autopilot lets go, and a run that exits after its race ends now if
    /// the race's own end did not ask already (a race given up, or one with no finish such as a pursuit challenge).
    pub(super) fn unattended_race_ended(&mut self) {
        self.release_autopilot();
        if self.unattended.plan.exit_after_race {
            race_run::request_exit(Ok(()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_with_no_options_counts_ticks_and_does_nothing_else() {
        let mut run = Unattended::default();
        run.note_tick(1.0 / 60.0);
        run.note_tick(1.0 / 60.0);
        assert_eq!(run.stamp().tick, 2);
        assert!((run.stamp().sim_time - 2.0 / 60.0).abs() < 1e-6);
        assert!(!run.pilot_engaged() && run.plan == RaceRunOptions::default());
        run.write(|_| unreachable!("no trace file was asked for, so no line is built"));
    }

    #[test]
    fn the_requested_race_keeps_the_events_own_values_unless_given() {
        let request = race_run::RaceRequest { event: "16.1.1".into(), opponents: None, laps: None };
        assert_eq!(options_of(&request), RaceOptions::default());
        let request = race_run::RaceRequest { opponents: Some(3), laps: Some(2), ..request };
        let options = options_of(&request);
        assert_eq!((options.opponents, options.laps), (Some(3), Some(2)));
        assert!(
            !options.career && !options.demo,
            "a comparison run is a plain race: no career result, no gate hopping"
        );
    }

    #[test]
    fn lines_go_to_the_trace_file_with_the_tick_and_the_simulated_time() {
        let path = std::env::temp_dir().join(format!("nfsmw-race-run-{}.jsonl", std::process::id()));
        let plan = RaceRunOptions { trace: Some(path.clone()), ..RaceRunOptions::default() };
        let mut run = Unattended { plan, ..Unattended::default() };
        run.note_tick(0.5);
        run.write(|stamp| Record::TimerStart { stamp, race_time: 0.0 });
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let line: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(line["kind"], "timer_start");
        assert_eq!((line["tick"].as_u64(), line["sim_time"].as_f64()), (Some(1), Some(0.5)));
    }
}
