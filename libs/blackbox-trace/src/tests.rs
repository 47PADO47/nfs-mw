use super::*;

fn json(capture: Capture) -> serde_json::Value {
    let mut bytes = Vec::new();
    capture.write_json(&mut bytes).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn nested_scopes_counters_and_escaped_bookmarks_export() {
    let recorder = Recorder::default();
    assert!(recorder.start(100));
    {
        let _outer = recorder.scope("frame");
        let _inner = recorder.scope("update");
        recorder.note("bookmark", "quote \" newline\n🌎");
        recorder.counter("GPU", &[("busy", 45.0), ("unknown", f64::NAN)]);
    }
    let trace = json(recorder.stop().unwrap());
    let events = trace["traceEvents"].as_array().unwrap();
    let frame = events.iter().find(|v| v["name"] == "frame").unwrap();
    let inner = events.iter().find(|v| v["name"] == "update").unwrap();
    assert!(frame["ts"].as_f64().unwrap() <= inner["ts"].as_f64().unwrap());
    assert!(frame["dur"].as_f64().unwrap() >= inner["dur"].as_f64().unwrap());
    let counter = events.iter().find(|v| v["name"] == "GPU").unwrap();
    assert_eq!(counter["args"]["busy"], 45.0);
    assert!(counter["args"].get("unknown").is_none());
    assert_eq!(events.iter().find(|v| v["name"] == "bookmark").unwrap()["args"]["detail"], "quote \" newline\n🌎");
}

#[test]
fn disabled_and_previous_session_guards_cannot_pollute_new_capture() {
    let recorder = Recorder::default();
    let disabled = recorder.scope("disabled");
    assert!(recorder.start(100));
    assert!(!recorder.start(100));
    let previous = recorder.scope("old");
    assert_eq!(recorder.stop().unwrap().events(), 0);
    assert!(recorder.start(100));
    drop(previous);
    drop(disabled);
    recorder.counter("invalid", &[("v", f64::INFINITY)]);
    assert_eq!(recorder.stop().unwrap().events(), 0);
}

#[test]
fn overflow_is_bounded_and_disclosed() {
    let recorder = Recorder::default();
    recorder.start(2);
    for _ in 0..10 {
        recorder.note("test", "event");
    }
    let trace = json(recorder.stop().unwrap());
    assert_eq!(trace["metadata"]["events"], 2);
    assert_eq!(trace["metadata"]["droppedEvents"], 8);
}

#[test]
fn worker_threads_share_clock_and_have_distinct_tracks() {
    let recorder = Recorder::default();
    recorder.start(100);
    std::thread::scope(|threads| {
        for _ in 0..3 {
            let recorder = &recorder;
            threads.spawn(move || {
                let _guard = recorder.scope("worker");
            });
        }
    });
    let trace = json(recorder.stop().unwrap());
    let workers: std::collections::BTreeSet<_> = trace["traceEvents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["name"] == "worker")
        .map(|e| e["tid"].as_u64().unwrap())
        .collect();
    assert_eq!(workers.len(), 3);
}

#[test]
fn totals_add_up_the_runs_of_a_scope_by_name() {
    let recorder = Recorder::default();
    assert!(recorder.start(100));
    for _ in 0..3 {
        let _scope = recorder.scope("step");
        std::thread::sleep(Duration::from_millis(2));
    }
    recorder.note("bookmark", "not a scope");
    let totals = recorder.stop().unwrap().totals();
    assert_eq!(totals.len(), 1);
    let step = totals["step"];
    assert_eq!(step.count, 3);
    assert!(step.total_us >= 6_000.0 && step.max_us >= 2_000.0 && step.max_us <= step.total_us);
}
