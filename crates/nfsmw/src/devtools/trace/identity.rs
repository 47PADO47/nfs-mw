//! Companion metadata export. Hashing and disk access happen only on the trace writer.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;

fn sha256(path: &Path) -> std::io::Result<String> {
    let mut input = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        hash.update(&buffer[..read]);
    }
}
fn build_identity(executable: &Path) -> Value {
    let metadata = executable.with_file_name("BUILD.json");
    let package = std::fs::metadata(&metadata)
        .ok()
        .filter(|m| m.len() <= 4 * 1024 * 1024)
        .and_then(|_| std::fs::read(&metadata).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let executable_hash = sha256(executable);
    // Sidecar revision/tree are declarations. The measured executable hash remains authoritative.
    json!({
        "executable_name": executable.file_name().map(|v| v.to_string_lossy()),
        "executable_sha256": executable_hash.as_ref().ok(),
        "executable_hash_error": executable_hash.as_ref().err().map(ToString::to_string),
        "package_metadata_sha256": sha256(&metadata).ok(),
        "declared_source_revision": package.as_ref().and_then(|p| p.get("source_revision")),
        "declared_source_tree_hash": package.as_ref().and_then(|p| p.get("source_tree_hash")),
        "declared_executable_sha256": package.as_ref().and_then(|p| p.get("sha256")),
        "package_version": env!("CARGO_PKG_VERSION"),
        "debug_assertions": cfg!(debug_assertions),
    })
}

pub(super) fn save_context(path: &Path, start: Instant, end: Instant, end_unix_ms: u64) -> Result<(), String> {
    // Snapshot history before hashing a large executable, so writer latency cannot evict old samples.
    let history = super::health::history(start, end);
    let elapsed_ms = end.saturating_duration_since(start).as_millis() as u64;
    let identity =
        std::env::current_exe().map(|p| build_identity(&p)).unwrap_or_else(|e| json!({"error": e.to_string()}));
    let context = json!({
        "schema_version": 1, "trace_file": path.file_name().map(|s| s.to_string_lossy()),
        "capture_start_unix_ms": end_unix_ms.saturating_sub(elapsed_ms),
        "capture_end_unix_ms": end_unix_ms,
        "build": identity, "health_history": history,
        "notes": [
            "History is sampled approximately once a second and bounded to 600 samples, including before F9.",
            "Negative relative_seconds precede capture. Monotonic-relative times are authoritative if the wall clock changes.",
            "Working set is resident process RAM; private commit includes nonresident memory. Neither is VRAM.",
            "Page fault count includes soft faults; it does not prove disk paging. Missing OS counters are omitted.",
            "Resources are main-thread snapshots; resource_age_ms exposes stale values during a blocked frame.",
            "The trace event buffer itself grows while recording; compare the pre-capture baseline and trace_recording intervals.",
            "GPU readbacks are delayed. Hitch markers identify observation time, not a proven cause."
        ]
    });
    let final_path = path.with_extension("context.json");
    let temporary = path.with_extension("context.json.partial");
    let result = (|| -> std::io::Result<()> {
        let output = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        let mut output = std::io::BufWriter::new(output);
        serde_json::to_writer_pretty(&mut output, &context)?;
        output.flush()?;
        drop(output);
        std::fs::rename(&temporary, final_path)
    })();
    result.map_err(|e| format!("Trace saved at {}; companion context failed: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn companion_export_is_valid_and_reports_an_existing_destination() {
        let root = std::env::temp_dir().join(format!("trace-context-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("capture.json");
        let at = Instant::now();
        save_context(&path, at, at, 12345).unwrap();
        let context: Value =
            serde_json::from_slice(&std::fs::read(path.with_extension("context.json")).unwrap()).unwrap();
        assert_eq!(context["schema_version"], 1);
        assert_eq!(context["capture_start_unix_ms"], 12345);
        assert_eq!(context["capture_end_unix_ms"], 12345);
        assert_eq!(context["build"]["executable_sha256"].as_str().unwrap().len(), 64);
        assert!(context["health_history"].is_array());
        std::fs::write(path.with_extension("context.json.partial"), b"preserve").unwrap();
        assert!(save_context(&path, at, at, 12345).is_err());
        assert_eq!(std::fs::read(path.with_extension("context.json.partial")).unwrap(), b"preserve");
        std::fs::remove_file(path.with_extension("context.json.partial")).unwrap();
        std::fs::remove_file(path.with_extension("context.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn hashes_bytes_and_keeps_unverified_package_declarations_distinct() {
        let root = std::env::temp_dir().join(format!("trace-identity-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let executable = root.join("fixture.bin");
        std::fs::write(&executable, b"abc").unwrap();
        std::fs::write(
            root.join("BUILD.json"),
            br#"{"source_revision":"fixture","source_tree_hash":"dirty-fixture","sha256":"wrong"}"#,
        )
        .unwrap();
        let identity = build_identity(&executable);
        assert_eq!(identity["executable_sha256"], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(identity["declared_source_tree_hash"], "dirty-fixture");
        assert_eq!(identity["declared_executable_sha256"], "wrong");
        std::fs::remove_file(root.join("BUILD.json")).unwrap();
        assert!(build_identity(&executable)["declared_source_revision"].is_null());
        std::fs::remove_file(executable).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
