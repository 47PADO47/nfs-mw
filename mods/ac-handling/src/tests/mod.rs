//! Unit tests: the test vectors of the specs, one file per spec, and the loaders and the switches.

mod control;
mod loader;
mod mapping_vectors;
mod tyre_force_vectors;
mod wheel_vectors;

/// `|a - b| <= max(abs, rel * |b|)`, with a message that says what differed.
#[track_caller]
pub fn assert_close(what: &str, got: f64, want: f64, rel: f64, abs: f64) {
    let allowed = abs.max(rel * want.abs());
    assert!((got - want).abs() <= allowed, "{what}: got {got}, want {want} (allowed {allowed})");
}

/// A scratch folder that goes away with the value.
pub struct TempDir(pub std::path::PathBuf);

impl TempDir {
    /// A new empty folder under the system's temp folder.
    pub fn new(tag: &str) -> TempDir {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("ac-handling-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create a scratch folder");
        TempDir(path)
    }

    /// Write `text` to `name` inside the folder (the folders in between are made).
    pub fn write(&self, name: &str, text: &str) {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create a folder");
        }
        std::fs::write(path, text).expect("write a file");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
