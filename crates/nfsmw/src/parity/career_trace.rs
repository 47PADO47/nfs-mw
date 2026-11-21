//! The career-flow trace: one JSON line per step of the career's story flow, from "start new career" on. Off unless
//! `--career-trace <file>` or `NFSMW_CAREER_TRACE=<file>` names a file (`docs/testing.md`, "Career-flow trace").
//!
//! It records what the flow did, not why: the front-end screens opened and closed, the movies and NIS scenes started and
//! ended, the races queued, started, finished and answered, the chain from one race to the next, the requests the world's
//! career sends the front end, the hand-offs between the front end and the world, and the career state whenever it
//! changes. Each line is stamped with the physics ticks run so far in the process and their simulated seconds (summed
//! over every world of the run, so the stamp only grows) and the wall time since the file was opened.
//!
//! Lines come from both the front end and the world, a few a second at most, so they are written and flushed where
//! they are raised: the trace is complete when the process ends, with no thread to wait for. A line that cannot be
//! written ends the trace; the game goes on.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::race_trace::Build;

/// The variable that names the trace file, as `--career-trace` does.
pub const CAREER_TRACE_ENV: &str = "NFSMW_CAREER_TRACE";
/// The layout's name, in the header line.
pub const SCHEMA: &str = "nfsmw-career-trace-v1";

struct Writer {
    path: PathBuf,
    out: File,
    opened: Instant,
    seq: u64,
}

/// The clock the lines are stamped with: physics ticks and their simulated seconds, over the whole run.
#[derive(Default)]
struct Clock {
    ticks: u64,
    sim_time: f64,
}

static TRACE: Mutex<Option<Writer>> = Mutex::new(None);
static CLOCK: Mutex<Clock> = Mutex::new(Clock { ticks: 0, sim_time: 0.0 });
static ON: AtomicBool = AtomicBool::new(false);
static EXIT_AFTER_FIRST_CAR: AtomicBool = AtomicBool::new(false);
static EXIT_AFTER_FIRST_VISIT: AtomicBool = AtomicBool::new(false);
static SKIP_CINEMATICS: AtomicBool = AtomicBool::new(false);
static START_AT_FIRST_CAR_LOT: AtomicBool = AtomicBool::new(false);

/// The trace file: the command line's over the environment's, read through `get`. An empty value is no file.
pub fn path_of(flag: Option<PathBuf>, get: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    flag.or_else(|| get(CAREER_TRACE_ENV).filter(|value| !value.trim().is_empty()).map(PathBuf::from))
}

/// Opens the trace at `path` (replacing a file there) and writes its header. Called once, when the command starts.
pub fn init(path: Option<&Path>) {
    let Some(path) = path else { return };
    let created = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| File::create(path));
    let out = match created {
        Ok(out) => out,
        Err(e) => return log::error!("career trace: cannot create {}: {e}", path.display()),
    };
    log::info!("career trace: recording into {}", path.display());
    if let Ok(mut trace) = TRACE.lock() {
        *trace = Some(Writer { path: path.to_owned(), out, opened: Instant::now(), seq: 0 });
    }
    ON.store(true, Ordering::Relaxed);
    let created_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    record("header", json!({"schema": SCHEMA, "created_unix_ms": created_unix_ms, "build": Build::current()}));
}

/// Whether the run records a trace. Callers that build a costly line test this first.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// One physics tick of `dt` simulated seconds ran, in whatever world is up.
pub fn note_tick(dt: f32) {
    if let Ok(mut clock) = CLOCK.lock() {
        clock.ticks += 1;
        clock.sim_time += f64::from(dt);
    }
}

/// Writes one line of `kind` with `fields` (an object; anything else is put under `value`).
pub fn record(kind: &str, fields: Value) {
    if !enabled() {
        return;
    }
    let (tick, sim_time) = CLOCK.lock().map_or((0, 0.0), |clock| (clock.ticks, clock.sim_time));
    let Ok(mut guard) = TRACE.lock() else { return };
    let Some(writer) = guard.as_mut() else { return };
    writer.seq += 1;
    let mut line = Map::new();
    line.insert("kind".into(), kind.into());
    line.insert("seq".into(), writer.seq.into());
    line.insert("tick".into(), tick.into());
    line.insert("sim_time".into(), json!(sim_time));
    line.insert("wall_ms".into(), json!(writer.opened.elapsed().as_secs_f64() * 1000.0));
    match fields {
        Value::Object(fields) => line.extend(fields),
        Value::Null => {}
        other => {
            line.insert("value".into(), other);
        }
    }
    let written = serde_json::to_vec(&Value::Object(line)).map_err(std::io::Error::other).and_then(|mut bytes| {
        bytes.push(b'\n');
        writer.out.write_all(&bytes).and_then(|()| writer.out.flush())
    });
    if let Err(e) = written {
        log::error!("career trace: writing {} failed, the trace ends here: {e}", writer.path.display());
        *guard = None;
        ON.store(false, Ordering::Relaxed);
    }
}

/// Asks the run to end once the first car of the career is bought (`--exit-after-first-car`).
pub fn set_exit_after_first_car(on: bool) {
    EXIT_AFTER_FIRST_CAR.store(on, Ordering::Relaxed);
}

/// The run's other options: `--exit-after-first-visit`, `--skip-cinematics` and `--career-start first-car-lot`.
pub fn set_run_options(exit_after_first_visit: bool, skip_cinematics: bool, start_at_first_car_lot: bool) {
    EXIT_AFTER_FIRST_VISIT.store(exit_after_first_visit, Ordering::Relaxed);
    SKIP_CINEMATICS.store(skip_cinematics, Ordering::Relaxed);
    START_AT_FIRST_CAR_LOT.store(start_at_first_car_lot, Ordering::Relaxed);
}

/// Whether movies and NIS scenes end as soon as they start (`--skip-cinematics`).
pub fn skip_cinematics() -> bool {
    SKIP_CINEMATICS.load(Ordering::Relaxed)
}

/// Whether a new career starts in the prologue's car lot (`--career-start first-car-lot`).
pub fn start_at_first_car_lot() -> bool {
    START_AT_FIRST_CAR_LOT.load(Ordering::Relaxed)
}

/// The first safe house visit handed the career to the safe house menu: a run that asked for it ends now.
pub fn first_visit_done() {
    if !EXIT_AFTER_FIRST_VISIT.load(Ordering::Relaxed) {
        return;
    }
    record("run_end", json!({"reason": "first_visit_done"}));
    log::info!("career run: the first safe house visit is over, the process ends");
    super::race_run::request_exit(Ok(()));
}

/// The first car of the career was bought: a run that asked for it ends now.
pub fn first_car_bought() {
    if !EXIT_AFTER_FIRST_CAR.load(Ordering::Relaxed) {
        return;
    }
    record("run_end", json!({"reason": "first_car_bought"}));
    log::info!("career run: the first car is bought, the process ends");
    super::race_run::request_exit(Ok(()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_wins_over_the_environment_and_an_empty_value_is_no_file() {
        let env = |value: &'static str| move |name: &str| (name == CAREER_TRACE_ENV).then(|| value.to_owned());
        assert_eq!(path_of(None, env("a.jsonl")), Some(PathBuf::from("a.jsonl")));
        assert_eq!(path_of(Some("b.jsonl".into()), env("a.jsonl")), Some(PathBuf::from("b.jsonl")));
        assert_eq!(path_of(None, env("  ")), None);
    }
}
