use super::*;

#[test]
fn json_escapes_paths_and_control_characters_and_preserves_words() {
    let record = Record {
        event: "voice_start",
        bank: "a\\b\"\n",
        params: &[-32767, 4096],
        success: Some(false),
        ..Default::default()
    };
    let line = record.json(7, 1.5);
    assert!(line.contains("\"bank\":\"a\\\\b\\\"\\n\""));
    assert!(line.contains("\"params\":[-32767, 4096]"));
    assert!(line.contains("\"success\":false"));
    assert!(line.contains("\"inputs\":[null,null,"));
    assert!(line.contains("\"trigger\":null,\"group\":null"));
    let tagged = Record { trigger: Some(4), group: Some("Sfx\"\\"), ..Default::default() }.json(8, 2.0);
    let decoded: serde_json::Value = serde_json::from_str(&tagged).unwrap();
    assert_eq!(decoded["trigger"], 4);
    assert_eq!(decoded["group"], "Sfx\"\\");
}

#[test]
fn trace_sessions_finish_and_existing_recorder_clones_follow_new_sessions() {
    let trace = Trace::default();
    let module_trace = trace.clone();
    let path = std::env::temp_dir().join(format!("nfsmw-audio-trace-{}.jsonl", std::process::id()));
    trace.start(&path).unwrap();
    module_trace.note("synthetic", "before stop");
    trace.stop();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("synthetic"));
    assert!(text.contains("\"event\":\"session_end\""));
    assert!(text.contains("\"dropped\":0"));
    assert!(!trace.enabled());
    std::fs::remove_file(path).unwrap();
}
