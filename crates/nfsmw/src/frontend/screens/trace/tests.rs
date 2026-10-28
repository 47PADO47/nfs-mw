use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use blackbox_feng::runtime::PackageCommandKind;
use blackbox_feng::{Outgoing, PackageId};
use serde_json::Value;

use super::{Trace, writer::Writer};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Files {
    root: PathBuf,
    checkout: PathBuf,
    output: PathBuf,
}

impl Files {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("nfsmw-ui-trace-{}-{id}", std::process::id()));
        let checkout = root.join("checkout");
        std::fs::create_dir_all(&checkout).unwrap();
        Self { output: root.join("trace.jsonl"), root, checkout }
    }

    fn records(&self) -> Vec<Value> {
        std::fs::read_to_string(&self.output).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect()
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn trace_preserves_message_order_and_labels_sound_as_a_request() {
    let files = Files::new();
    let writer = Writer::create(&files.output, &files.checkout).unwrap();
    let mut trace = Trace::new(writer);
    trace.begin_frame(1.0 / 60.0, 8, 4, false);
    trace.outgoing(&[
        Outgoing::Sound { message: 42, package: PackageId(2), from: Some(9) },
        Outgoing::Game { message: 43, package: PackageId(2), from: None },
        Outgoing::Package { kind: PackageCommandKind::Pop, name: String::new(), from: PackageId(2) },
    ]);
    drop(trace);
    let records = files.records();
    let kinds: Vec<_> = records.iter().map(|record| record["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["header", "input", "sound_request", "game_message", "package_request", "end"]);
    assert_eq!(records[1]["data"]["host_pad"], 8);
    assert_eq!(records[1]["data"]["menu_pad"], 4);
    assert_eq!(records[2]["data"]["from_guid"], 9);
    assert_eq!(records[0]["data"]["sound_evidence"], "runtime_request_not_playback");
    for pair in records.windows(2) {
        assert!(pair[0]["seq"].as_u64().unwrap() < pair[1]["seq"].as_u64().unwrap());
        assert!(pair[0]["elapsed_ns"].as_u64().unwrap() <= pair[1]["elapsed_ns"].as_u64().unwrap());
    }
}

#[test]
fn input_release_and_gating_are_kept_without_repeating_unchanged_input() {
    let files = Files::new();
    let writer = Writer::create(&files.output, &files.checkout).unwrap();
    let mut trace = Trace::new(writer);
    for _ in 0..10 {
        trace.begin_frame(1.0 / 60.0, 1, 0, true);
    }
    trace.begin_frame(1.0 / 60.0, 0, 0, false);
    trace.begin_frame(1.0 / 60.0, 1, 1, false);
    drop(trace);
    let records = files.records();
    let inputs: Vec<_> = records.iter().filter(|record| record["kind"] == "input").collect();
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0]["data"]["await_release"], true);
    assert_eq!(inputs[1]["frame"], 11);
    assert_eq!(inputs[2]["data"]["menu_pad"], 1);
}

#[test]
fn writer_rejects_checkout_output_and_never_overwrites_an_existing_capture() {
    let files = Files::new();
    assert!(Writer::create(&files.checkout.join("trace.jsonl"), &files.checkout).is_err());
    assert!(Writer::create(std::path::Path::new("trace.jsonl"), &files.checkout).is_err());
    let writer = Writer::create(&files.output, &files.checkout).unwrap();
    drop(writer);
    let before = std::fs::read(&files.output).unwrap();
    assert!(Writer::create(&files.output, &files.checkout).is_err());
    assert_eq!(std::fs::read(&files.output).unwrap(), before);
}
