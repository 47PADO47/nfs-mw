//! The pursuit trace recorder: 20 Hz police samples and the events between them, written as JSON lines in the layout
//! of the private original-game police recorder so one analysis can read both (`records.rs`). Started from F5
//! ("Record pursuit trace"), the `extra trace on` console verb, or `NFSMW_PURSUIT_TRACE=<folder>` when the world opens.
//! Each recording is a new folder `<folder>/<UTC time>-remake/` with `samples.jsonl`, `events.jsonl` and
//! `session.json`. Without the variable the folder is `pursuit-traces` in the app's log folder.
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde_json::json;

use super::{WorldScene, space};

mod diff;
mod records;
mod session;
mod writer;

use records::{Event, LIVE, Player, Sample, Stamp};
use writer::{Record, Writer};

/// The variable that names the trace folder and starts recording when the world opens.
const ENV: &str = "NFSMW_PURSUIT_TRACE";
/// Milliseconds between samples.
const SAMPLE_MS: f64 = 1000.0 / session::SAMPLE_HZ as f64;

/// The recorder of one world.
#[derive(Default)]
pub(super) struct PursuitTrace {
    env_checked: bool,
    active: Option<Active>,
    /// The writer of the last recording, kept until it has finished.
    closing: Option<Writer>,
    last_session: Option<PathBuf>,
    error: Option<String>,
}

struct Active {
    writer: Writer,
    dir: PathBuf,
    started: Instant,
    next_sample_ms: f64,
    sim_time: f64,
    sequence: u64,
    tracker: diff::Tracker,
    speech_seen: u32,
    session: session::Session,
}

/// The folder sessions go into: the variable's, or `pursuit-traces` in the log folder.
fn root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    game_install::log_dir(&nfsmw_data::game::SPEC).map(|dir| dir.join("pursuit-traces"))
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl WorldScene {
    /// Starts or stops recording.
    pub(super) fn set_pursuit_trace(&mut self, on: bool) -> Result<()> {
        if !on {
            self.stop_pursuit_trace("stopped from the extra options");
            return Ok(());
        }
        if self.pursuit_trace.active.is_some() {
            return Ok(());
        }
        let root = root().context("No folder for the trace: set NFSMW_PURSUIT_TRACE")?;
        let (stamp, iso) = session::utc(unix_ms() / 1000);
        let dir = root.join(format!("{stamp}-remake"));
        let writer = Writer::start(dir.clone()).context("The trace writer could not start")?;
        let meta = session::Session::new(iso, self.dir.root().display().to_string(), self.trace_settings());
        writer.send(Record::Session(Box::new(meta.clone())));
        log::info!("pursuit trace: recording into {}", dir.display());
        self.pursuit_trace.error = None;
        self.pursuit_trace.last_session = Some(dir.clone());
        self.pursuit_trace.active = Some(Active {
            writer,
            dir,
            started: Instant::now(),
            next_sample_ms: 0.0,
            sim_time: 0.0,
            sequence: 0,
            tracker: diff::Tracker::default(),
            // The first snapshot skips the speech events from before the recording and moves on to the newest.
            speech_seen: u32::MAX,
            session: meta,
        });
        Ok(())
    }

    fn stop_pursuit_trace(&mut self, reason: &str) {
        let Some(mut active) = self.pursuit_trace.active.take() else { return };
        active.session.stop_reason = Some(reason.into());
        active.writer.send(Record::Session(Box::new(active.session.clone())));
        log::info!("pursuit trace: {reason}; written to {}", active.dir.display());
        // Dropping the sender ends the thread once it has written the queue; it is not waited for.
        self.pursuit_trace.closing = Some(active.writer);
    }

    /// The settings the session records.
    fn trace_settings(&self) -> serde_json::Value {
        let traffic = &self.traffic_setting;
        let testing = &self.extra.testing;
        json!({
            "physics_rate_hz": self.tuning.physics_rate_hz,
            "game_speed": self.tuning.game_speed,
            "time_scale_allowed": self.rules.time_scale_allowed,
            "traffic_cars": traffic.cars,
            "cop_share": format!("{:?}", traffic.cop_share),
            "traffic_manual": self.traffic_manual,
            "traffic_lights": traffic.lights,
            "traffic_avoidance": traffic.avoidance,
            "traffic_policy": serde_json::to_value(traffic.policy).ok(),
            "pursuit_options": format!("{:?}", traffic.pursuit),
            "track": self.track,
            "testing": {"traffic_off": testing.traffic_off, "police_off": testing.police_off,
                "lock_heat": testing.lock_heat, "heat": testing.heat, "paused": testing.paused},
        })
    }

    /// A reference jump landed: the session records the place, the pose and the heat.
    pub(super) fn pursuit_trace_landed(&mut self, number: i32, at: [f32; 3], heading_deg: f32, heat: Option<u8>) {
        let name = self.extra.bounty_spots.iter().find(|s| s.number == number).map(|s| s.name.clone());
        let Some(active) = self.pursuit_trace.active.as_mut() else { return };
        let reference = json!({
            "bounty_spot": number,
            "bounty_label": crate::extra_options::reference::label(number),
            "bounty_name": name,
            "start_heat": heat,
            "pursuit_started_on_landing": heat.is_some(),
            "landed_at_render": at,
            "landed_at": space::to_physics(glam::Vec3::from_array(at)),
            "heading_render_deg": heading_deg,
            "heading_rule": "the bounty marker's Position and Rotation, as the rap sheet jump (garage-exit placement); the road-based choice only without a Rotation",
        });
        active.session.reference = Some(reference.clone());
        active.writer.send(Record::Session(Box::new(active.session.clone())));
        let stamp = Stamp {
            time_ms: active.started.elapsed().as_secs_f64() * 1000.0,
            wall_time_ms: unix_ms(),
            sim_time: active.sim_time,
        };
        active.sequence += 1;
        active.writer.send(Record::Event(Box::new(Event {
            kind: "reference_jump",
            sequence: active.sequence,
            stamp,
            context: LIVE,
            id: None,
            generation: None,
            vehicle: None,
            role: None,
            pursuit: None,
            state: None,
            remake: reference,
        })));
    }

    /// Each frame: the env-var start, a sample when one is due, and the F5 panel's view.
    pub(super) fn update_pursuit_trace(&mut self, sim_dt: f32) {
        if !std::mem::replace(&mut self.pursuit_trace.env_checked, true)
            && std::env::var_os(ENV).is_some_and(|v| !v.is_empty())
            && let Err(e) = self.set_pursuit_trace(true)
        {
            self.pursuit_trace.error = Some(format!("{e:#}"));
        }
        self.sample_pursuit_trace(sim_dt);
        self.publish_trace_view();
    }

    fn sample_pursuit_trace(&mut self, sim_dt: f32) {
        let Some(active) = self.pursuit_trace.active.as_mut() else { return };
        active.sim_time += f64::from(sim_dt.max(0.0));
        let time_ms = active.started.elapsed().as_secs_f64() * 1000.0;
        if time_ms < active.next_sample_ms {
            return;
        }
        // A late frame does not make a burst of samples: the next is due one period after this one at the latest.
        active.next_sample_ms = (active.next_sample_ms + SAMPLE_MS).max(time_ms);
        let stamp = Stamp { time_ms, wall_time_ms: unix_ms(), sim_time: active.sim_time };
        let player = self.players.lead().filter(|d| d.has_sim()).map(|drive| {
            let view = drive.player_view();
            Player {
                position: space::to_physics(view.position),
                velocity: space::to_physics(view.velocity),
                forward: space::to_physics(view.forward),
                speed: view.speed.abs(),
            }
        });
        let Some(traffic) = self.traffic.as_ref() else { return };
        let frame = traffic.trace_frame(&mut active.speech_seen);
        let pursuit = active.tracker.pursuit_id(&frame);
        for found in active.tracker.events(&frame, player.as_ref().map(|p| p.position)) {
            active.sequence += 1;
            let pursuit = found.member.then(|| pursuit.clone()).flatten();
            let cop = found.cop.as_ref();
            let state = cop.map(|c| records::cop(c, found.member, pursuit.as_deref()));
            active.writer.send(Record::Event(Box::new(Event {
                kind: found.kind,
                sequence: active.sequence,
                stamp,
                context: LIVE,
                id: cop.map(|c| records::cop_id(c.key)),
                generation: cop.map(|_| 1),
                vehicle: cop.map(|c| format!("remake-{}", c.key)),
                role: state.as_ref().map(records::Cop::role),
                pursuit: pursuit.clone(),
                state,
                remake: found.detail,
            })));
        }
        let pursuit = active.tracker.pursuit_id(&frame);
        let paused = self.extra.testing.paused || sim_dt <= 0.0;
        let sample = Sample {
            kind: "sample",
            stamp,
            heat: Some(frame.heat),
            cops: frame
                .cops
                .iter()
                .map(|c| records::cop(c, records::in_pursuit(c, &frame), pursuit.as_deref()))
                .collect(),
            player,
            context: LIVE,
            remake: records::sample_extras(&frame, self.extra.testing.heat, paused),
        };
        active.writer.send(Record::Sample(Box::new(sample)));
    }

    fn publish_trace_view(&mut self) {
        let trace = &mut self.pursuit_trace;
        if trace.closing.as_ref().is_some_and(Writer::finished)
            && let Some(writer) = trace.closing.take()
        {
            trace.error = writer.counters.error().or(trace.error.take());
        }
        let view = &mut self.extra.trace;
        view.recording = trace.active.is_some();
        view.root = root();
        view.session = trace.last_session.clone();
        view.error = trace.error.clone();
        let Some(writer) = trace.active.as_ref().map(|a| &a.writer).or(trace.closing.as_ref()) else { return };
        let counters = &writer.counters;
        view.samples = counters.samples.load(Ordering::Relaxed);
        view.events = counters.events.load(Ordering::Relaxed);
        view.dropped = counters.dropped.load(Ordering::Relaxed) + counters.over_budget.load(Ordering::Relaxed);
        view.error = counters.error().or(view.error.take());
    }
}
