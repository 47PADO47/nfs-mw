//! Bounded process health history. OS queries run on a worker; the frame only sends small snapshots.
#[cfg(windows)]
mod windows;
use crate::app::Host;
use blackbox_gfx::RenderBackend;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HISTORY: usize = 600;
static MONITOR: OnceLock<Option<Monitor>> = OnceLock::new();

#[derive(Clone)]
struct Resources {
    at: Instant,
    values: Value,
}
#[derive(Clone)]
struct Snapshot {
    at: Instant,
    unix_ms: u64,
    sample: u64,
    memory: Vec<(&'static str, f64)>,
    recording: bool,
    resources: Option<Resources>,
}
struct Monitor {
    send: mpsc::SyncSender<Resources>,
    history: Arc<Mutex<VecDeque<Snapshot>>>,
}

pub(super) fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn monitor() -> Option<&'static Monitor> {
    MONITOR
        .get_or_init(|| {
            let (send, receive) = mpsc::sync_channel(1);
            let history = Arc::new(Mutex::new(VecDeque::new()));
            let output = Arc::clone(&history);
            std::thread::Builder::new()
                .name("performance-health".into())
                .spawn(move || {
                    let mut resources = None;
                    let mut sample = 0;
                    let mut next = Instant::now() + Duration::from_secs(1);
                    loop {
                        match receive.recv_timeout(next.saturating_duration_since(Instant::now())) {
                            Ok(value) => resources = Some(value),
                            Err(mpsc::RecvTimeoutError::Disconnected) => return,
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                        if Instant::now() < next {
                            continue;
                        }
                        next = Instant::now() + Duration::from_secs(1);
                        sample += 1;
                        #[cfg(windows)]
                        let memory = windows::sample();
                        #[cfg(not(windows))]
                        let memory = Vec::new();
                        let snapshot = Snapshot {
                            at: Instant::now(),
                            unix_ms: unix_ms(),
                            sample,
                            memory,
                            recording: blackbox_trace::global().recording(),
                            resources: resources.clone(),
                        };
                        if let Ok(mut history) = output.lock() {
                            retain(&mut history, snapshot);
                        }
                    }
                })
                .ok()?;
            Some(Monitor { send, history })
        })
        .as_ref()
}
fn retain(history: &mut VecDeque<Snapshot>, snapshot: Snapshot) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(snapshot);
}

pub(super) fn observe(renderer: &dyn RenderBackend, host: &Host) {
    let Some(monitor) = monitor() else { return };
    let stats = renderer.stats();
    let shadows = renderer.experiment_stats();
    let camera = host.scene.camera_readout();
    let values = json!({
        "meshes": stats.meshes, "textures": stats.textures,
        "surface_effect_vertices": stats.effect_capacities[0],
        "particle_effect_vertices": stats.effect_capacities[1], "streak_vertices": stats.streak_capacity,
        "shadow_cascades": shadows.cascades,
        "shadow_draws": shadows.shadow_draws,
        "shadow_instances": shadows.shadow_instances,
        "shadow_cached_cascades": shadows.shadow_cached_cascades,
        "shadow_map_mib": shadows.map_mib,
        "scene_status": host.scene.status(),
        "camera_position": camera.as_ref().map(|v| v.position),
        "camera_forward": camera.as_ref().map(|v| v.forward),
        "scene_changes": host.scene_changes,
    });
    let _ = monitor.send.try_send(Resources { at: Instant::now(), values });
}

pub(super) fn record(last: &mut u64) {
    let Some(monitor) = monitor() else { return };
    let Ok(history) = monitor.history.try_lock() else { return };
    let Some(sample) = history.back().filter(|s| s.sample != *last) else { return };
    *last = sample.sample;
    let trace = blackbox_trace::global();
    let mut memory = sample.memory.clone();
    memory.extend([("sample_id", sample.sample as f64), ("sample_age_ms", sample.at.elapsed().as_secs_f64() * 1000.0)]);
    trace.counter("Process memory (OS sample)", &memory);
    if let Some(resources) = &sample.resources {
        let mut values: Vec<_> = [
            "meshes",
            "textures",
            "surface_effect_vertices",
            "particle_effect_vertices",
            "streak_vertices",
            "shadow_cascades",
            "shadow_draws",
            "shadow_instances",
            "shadow_cached_cascades",
            "shadow_map_mib",
            "scene_changes",
        ]
        .into_iter()
        .filter_map(|name| Some((name, resources.values[name].as_f64()?)))
        .collect();
        values.push(("sample_age_ms", resources.at.elapsed().as_secs_f64() * 1000.0));
        trace.counter("Resident resources", &values);
        trace.note("Scene snapshot", &resources.values.to_string());
    }
}

/// Called only by the export worker. Relative time uses Instant; wall clock is for external correlation.
pub(super) fn history(start: Instant, end: Instant) -> Value {
    let Some(monitor) = MONITOR.get().and_then(Option::as_ref) else { return json!([]) };
    let Ok(history) = monitor.history.lock() else { return json!([]) };
    Value::Array(
        history
            .iter()
            .filter(|s| s.at <= end)
            .map(|s| {
                let seconds = match s.at >= start {
                    true => s.at.duration_since(start).as_secs_f64(),
                    false => -start.duration_since(s.at).as_secs_f64(),
                };
                let memory: serde_json::Map<_, _> =
                    s.memory.iter().map(|(name, value)| ((*name).into(), json!(value))).collect();
                json!({"relative_seconds": seconds, "unix_ms": s.unix_ms, "sample_id": s.sample,
            "memory": memory, "trace_recording": s.recording, "resources": s.resources.as_ref().map(|r| &r.values),
            "resource_age_ms": s.resources.as_ref().map(|r| s.at.saturating_duration_since(r.at).as_secs_f64()*1000.0)})
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_keeps_memory_history_when_resource_producer_stops() {
        let monitor = monitor().expect("health worker");
        assert!(monitor.send.try_send(Resources { at: Instant::now(), values: json!({"meshes": 123}) }).is_ok());
        let until = Instant::now() + Duration::from_secs(6);
        loop {
            if monitor.history.lock().unwrap().len() >= 2 {
                break;
            }
            assert!(Instant::now() < until, "health samples continue without another frame");
            std::thread::sleep(Duration::from_millis(20));
        }
        let start = Instant::now();
        let samples = history(start, start);
        let samples = samples.as_array().unwrap();
        assert!(samples.len() >= 2);
        assert!(samples.iter().all(|s| s["relative_seconds"].as_f64().unwrap() <= 0.0));
        assert_eq!(samples.last().unwrap()["resources"]["meshes"], 123);
        assert!(samples.last().unwrap()["resource_age_ms"].as_f64().unwrap() >= 1000.0);
        #[cfg(windows)]
        assert!(samples.last().unwrap()["memory"]["private_commit_bytes"].as_f64().unwrap() > 0.0);
    }
    #[test]
    fn history_retains_only_the_latest_ten_minutes_of_samples() {
        let mut history = VecDeque::new();
        for sample in 0..HISTORY as u64 + 5 {
            retain(
                &mut history,
                Snapshot {
                    at: Instant::now(),
                    unix_ms: sample,
                    sample,
                    memory: vec![],
                    recording: false,
                    resources: None,
                },
            );
        }
        assert_eq!(history.len(), HISTORY);
        assert_eq!(history.front().unwrap().sample, 5);
        assert_eq!(history.back().unwrap().sample, HISTORY as u64 + 4);
    }
}
